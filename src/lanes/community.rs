use anyhow::{bail, Context, Result};
use console::style;

use super::{
    escape_toml_value, format_lane_toml, load_lane_config, FledgeFileWithLanes,
    LANES_IMPORT_SCHEMA, LANES_SEARCH_SCHEMA,
};
use crate::trust::{determine_trust_tier, determine_trust_tier_from_owner};

pub(crate) fn search_lanes(keyword: Option<&str>, author: Option<&str>, json: bool) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let query = crate::search::build_search_query_ex(keyword, author, "fledge-lane");

    let sp = crate::spinner::Spinner::start("Searching GitHub for community lanes:");

    let body = crate::github::github_api_get(
        "/search/repositories",
        token.as_deref(),
        &[("q", &query), ("sort", "stars"), ("per_page", "30")],
    )
    .context("searching GitHub for lane repos")?;

    sp.finish();

    let results = crate::search::parse_search_response(&body)?;

    if results.is_empty() {
        if json {
            let result = crate::envelope::resource(
                LANES_SEARCH_SCHEMA,
                "results",
                Vec::<serde_json::Value>::new(),
            );
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "{} No community lanes found{}.",
                style("*").cyan().bold(),
                keyword
                    .map(|q| format!(" matching '{q}'"))
                    .unwrap_or_default()
            );
        }
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let tier = determine_trust_tier_from_owner(&r.owner);
                r.to_json(tier.label())
            })
            .collect();
        let result = crate::envelope::resource(LANES_SEARCH_SCHEMA, "results", entries);
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("{}\n", style("Community lanes on GitHub:").bold());
    let max_name = results
        .iter()
        .map(|r| r.full_name().len())
        .max()
        .unwrap_or(0);
    for r in &results {
        let tier = determine_trust_tier_from_owner(&r.owner);
        let stars = crate::search::format_stars(r.stars);
        let desc = if r.description.chars().count() > 60 {
            let truncated: String = r.description.chars().take(57).collect();
            format!("{truncated}...")
        } else {
            r.description.clone()
        };
        let topic_str = if r.topics.is_empty() {
            String::new()
        } else {
            format!(" [{}]", r.topics.join(", "))
        };
        println!(
            "  {:<width$}  [{}]  {}  {}{}",
            style(&r.full_name()).green(),
            tier.styled_label(),
            style(format!("(⭐ {})", stars)).dim(),
            style(&desc).dim(),
            style(&topic_str).cyan(),
            width = max_name,
        );
    }
    println!(
        "\n{}",
        style("Import with: fledge lane import <owner/repo[/path]>").dim()
    );

    Ok(())
}

pub(crate) fn import_lanes(source: &str, yes: bool, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let local_path = cwd.join("fledge.toml");

    if !local_path.exists() {
        bail!(
            "No fledge.toml found. Run {} first.",
            style("fledge run --init").cyan()
        );
    }

    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let (owner, repo, subpath, git_ref) = parse_import_source(source);

    let display_source = format!(
        "{}/{}{}{}",
        owner,
        repo,
        subpath
            .as_ref()
            .map(|p| format!("/{p}"))
            .unwrap_or_default(),
        git_ref
            .as_ref()
            .map(|r| format!("@{r}"))
            .unwrap_or_default()
    );

    let tier = determine_trust_tier(&display_source);
    if !json {
        println!(
            "\n{} Importing lanes from: {} [{}]",
            style("!").yellow().bold(),
            style(&display_source).cyan(),
            tier.styled_label()
        );
        if tier != crate::trust::TrustTier::Official {
            println!(
                "  {} Lanes can execute arbitrary commands on your system.",
                style("*").yellow()
            );
            println!(
                "  {} Only import lanes from sources you trust.\n",
                style("*").yellow()
            );
        }
    }

    if !yes && tier != crate::trust::TrustTier::Official {
        if !crate::utils::is_interactive() {
            bail!(
                "Importing community lanes requires confirmation in non-interactive mode.\n  \
                 Use --yes to acknowledge that lanes can execute arbitrary commands."
            );
        }
        let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!("Import lanes from '{display_source}'?"))
            .default(false)
            .interact()?;
        if !confirm {
            bail!("Lane import cancelled.");
        }
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start(&format!(
            "Fetching lanes from {}:",
            display_source,
        )))
    };

    let ref_param = git_ref.as_deref().unwrap_or("HEAD");
    let remote_path = match &subpath {
        Some(p) => format!("{p}/fledge.toml"),
        None => "fledge.toml".to_string(),
    };
    let body = crate::github::github_api_get(
        &format!("/repos/{owner}/{repo}/contents/{remote_path}"),
        token.as_deref(),
        &[("ref", ref_param)],
    )
    .context(format!("fetching {remote_path} from remote repo"))?;

    if let Some(s) = sp {
        s.finish();
    }

    let content_b64 = body
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("Remote repo has no fledge.toml or it's not a file"))?;

    let cleaned: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64_decode(&cleaned).context("decoding fledge.toml content")?;
    let remote_content = String::from_utf8(decoded).context("fledge.toml is not valid UTF-8")?;

    let remote_config: FledgeFileWithLanes =
        toml::from_str(&remote_content).context("parsing remote fledge.toml")?;

    if remote_config.lanes.is_empty() {
        bail!("Remote repo has no [lanes] defined in fledge.toml.");
    }

    let existing = load_lane_config()?;

    let mut imported_lanes = Vec::new();
    let mut skipped = Vec::new();
    let mut import_content = String::new();

    import_content.push_str(&format!("# Imported from {display_source}\n\n"));

    for (task_name, task_def) in &remote_config.tasks {
        if existing.tasks.contains_key(task_name) {
            continue;
        }
        let cmd = escape_toml_value(task_def.cmd());
        import_content.push_str(&format!("[tasks.{task_name}]\ncmd = \"{cmd}\"\n\n"));
    }

    for (lane_name, lane) in &remote_config.lanes {
        if existing.lanes.contains_key(lane_name) {
            skipped.push(lane_name.clone());
            continue;
        }
        import_content.push_str(&format_lane_toml(lane_name, lane));
        imported_lanes.push(lane_name.clone());
    }

    let safe_name = format!(
        "{}-{}{}",
        owner.to_lowercase(),
        repo.to_lowercase(),
        subpath
            .as_ref()
            .map(|p| format!("-{}", p.replace('/', "-").to_lowercase()))
            .unwrap_or_default()
    );
    let relative_file = format!(".fledge/lanes/{safe_name}.toml");

    if imported_lanes.is_empty() {
        if json {
            let result = serde_json::json!({
                "schema_version": LANES_IMPORT_SCHEMA,
                "action": "import",
                "source": display_source,
                "trust_tier": tier.label(),
                "imported": [],
                "skipped": skipped,
                "file": relative_file,
                "written": false,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "{} All lanes from {} already exist locally ({})",
                style("*").cyan().bold(),
                display_source,
                skipped.join(", ")
            );
        }
        return Ok(());
    }

    let lanes_dir = cwd.join(".fledge").join("lanes");
    std::fs::create_dir_all(&lanes_dir).context("creating .fledge/lanes directory")?;

    let import_path = lanes_dir.join(format!("{safe_name}.toml"));
    std::fs::write(&import_path, import_content.trim_start()).context("writing imported lanes")?;

    if json {
        let result = serde_json::json!({
            "schema_version": LANES_IMPORT_SCHEMA,
            "action": "import",
            "source": display_source,
            "trust_tier": tier.label(),
            "imported": imported_lanes,
            "skipped": skipped,
            "file": relative_file,
            "written": true,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} Imported {} lane(s) from {}",
            style("✅").green().bold(),
            imported_lanes.len(),
            display_source
        );
        for name in &imported_lanes {
            println!("  {} {}", style("+").green(), style(name).cyan());
        }
        println!(
            "  {} Saved to {}",
            style("→").dim(),
            style(&relative_file).cyan()
        );
        if !skipped.is_empty() {
            println!(
                "  {} Skipped (already exist): {}",
                style("*").dim(),
                skipped.join(", ")
            );
        }
    }

    Ok(())
}

pub(crate) fn parse_import_source(
    source: &str,
) -> (String, String, Option<String>, Option<String>) {
    let source = source
        .strip_prefix("https://github.com/")
        .unwrap_or(source)
        .trim_end_matches(".git");

    let (path, git_ref) = if let Some((p, r)) = source.split_once('@') {
        (p, Some(r.to_string()))
    } else {
        (source, None)
    };

    let parts: Vec<&str> = path.splitn(3, '/').collect();
    let owner = parts.first().unwrap_or(&"").to_string();
    let repo = parts.get(1).unwrap_or(&"").to_string();
    let subpath = parts
        .get(2)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    (owner, repo, subpath, git_ref)
}

pub(crate) fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(input))
        .context("invalid base64 input")
}
