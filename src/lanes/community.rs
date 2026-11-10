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
    let display_source = import_display_source(&owner, &repo, &subpath, &git_ref);

    let tier = determine_trust_tier(&display_source);
    if !json {
        print_import_warning(&display_source, tier);
    }
    confirm_import(yes, tier, &display_source)?;

    let remote_config = fetch_remote_lane_config(
        &owner,
        &repo,
        &subpath,
        &git_ref,
        token.as_deref(),
        &display_source,
        json,
    )?;

    let existing = load_lane_config()?;
    let plan = plan_lane_import(&remote_config, &existing, &display_source);

    let safe_name = import_target_name(&owner, &repo, &subpath);
    let relative_file = format!(".fledge/lanes/{safe_name}.toml");

    if plan.imported.is_empty() {
        if json {
            print_import_envelope(&display_source, tier, &plan, &relative_file, false)?;
        } else {
            println!(
                "{} All lanes from {} already exist locally ({})",
                style("*").cyan().bold(),
                display_source,
                plan.skipped.join(", ")
            );
        }
        return Ok(());
    }

    let lanes_dir = cwd.join(".fledge").join("lanes");
    std::fs::create_dir_all(&lanes_dir).context("creating .fledge/lanes directory")?;

    let import_path = lanes_dir.join(format!("{safe_name}.toml"));
    std::fs::write(&import_path, plan.content.trim_start()).context("writing imported lanes")?;

    if json {
        print_import_envelope(&display_source, tier, &plan, &relative_file, true)?;
    } else {
        print_import_summary(&display_source, &plan, &relative_file);
    }

    Ok(())
}

/// What an import will write: the TOML for the new tasks and lanes, plus the
/// lane names imported and skipped because they already exist locally.
struct LaneImportPlan {
    content: String,
    imported: Vec<String>,
    skipped: Vec<String>,
}

/// `owner/repo[/subpath][@ref]`, the form shown to the user and used to pick
/// the trust tier.
fn import_display_source(
    owner: &str,
    repo: &str,
    subpath: &Option<String>,
    git_ref: &Option<String>,
) -> String {
    format!(
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
    )
}

/// The banner naming the source and its trust tier, with the arbitrary-command
/// warning for anything that is not official. Caller gates on `!json`.
fn print_import_warning(display_source: &str, tier: crate::trust::TrustTier) {
    println!(
        "\n{} Importing lanes from: {} [{}]",
        style("!").yellow().bold(),
        style(display_source).cyan(),
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

/// The confirmation gate for non-official sources. `--yes` skips it; without a
/// TTY it refuses rather than importing unconfirmed.
fn confirm_import(yes: bool, tier: crate::trust::TrustTier, display_source: &str) -> Result<()> {
    if yes || tier == crate::trust::TrustTier::Official {
        return Ok(());
    }
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
    Ok(())
}

/// Fetch the remote `fledge.toml` through the GitHub contents API, decode it
/// and parse it. Refuses a remote config that defines no lanes.
fn fetch_remote_lane_config(
    owner: &str,
    repo: &str,
    subpath: &Option<String>,
    git_ref: &Option<String>,
    token: Option<&str>,
    display_source: &str,
    json: bool,
) -> Result<FledgeFileWithLanes> {
    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start(&format!(
            "Fetching lanes from {}:",
            display_source,
        )))
    };

    let ref_param = git_ref.as_deref().unwrap_or("HEAD");
    let remote_path = match subpath {
        Some(p) => format!("{p}/fledge.toml"),
        None => "fledge.toml".to_string(),
    };
    let body = crate::github::github_api_get(
        &format!("/repos/{owner}/{repo}/contents/{remote_path}"),
        token,
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
    Ok(remote_config)
}

/// Decide what to import. Tasks and lanes that already exist locally are left
/// alone; skipped lanes are reported, skipped tasks are not. Pure.
fn plan_lane_import(
    remote: &FledgeFileWithLanes,
    existing: &FledgeFileWithLanes,
    display_source: &str,
) -> LaneImportPlan {
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut content = String::new();

    content.push_str(&format!("# Imported from {display_source}\n\n"));

    for (task_name, task_def) in &remote.tasks {
        if existing.tasks.contains_key(task_name) {
            continue;
        }
        let cmd = escape_toml_value(task_def.cmd());
        content.push_str(&format!("[tasks.{task_name}]\ncmd = \"{cmd}\"\n\n"));
    }

    for (lane_name, lane) in &remote.lanes {
        if existing.lanes.contains_key(lane_name) {
            skipped.push(lane_name.clone());
            continue;
        }
        content.push_str(&format_lane_toml(lane_name, lane));
        imported.push(lane_name.clone());
    }

    LaneImportPlan {
        content,
        imported,
        skipped,
    }
}

/// The file stem under `.fledge/lanes/` for an import: lowercased
/// `owner-repo[-sub-path]`. Pure.
fn import_target_name(owner: &str, repo: &str, subpath: &Option<String>) -> String {
    format!(
        "{}-{}{}",
        owner.to_lowercase(),
        repo.to_lowercase(),
        subpath
            .as_ref()
            .map(|p| format!("-{}", p.replace('/', "-").to_lowercase()))
            .unwrap_or_default()
    )
}

/// Print the `lanes import --json` envelope. `written` is false when every
/// lane was skipped and no file was written.
fn print_import_envelope(
    display_source: &str,
    tier: crate::trust::TrustTier,
    plan: &LaneImportPlan,
    relative_file: &str,
    written: bool,
) -> Result<()> {
    let result = crate::envelope::action(
        LANES_IMPORT_SCHEMA,
        "import",
        serde_json::json!({
            "source": display_source,
            "trust_tier": tier.label(),
            "imported": plan.imported,
            "skipped": plan.skipped,
            "file": relative_file,
            "written": written,
        }),
    );
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// The human summary after a successful import.
fn print_import_summary(display_source: &str, plan: &LaneImportPlan, relative_file: &str) {
    println!(
        "{} Imported {} lane(s) from {}",
        style("✅").green().bold(),
        plan.imported.len(),
        display_source
    );
    for name in &plan.imported {
        println!("  {} {}", style("+").green(), style(name).cyan());
    }
    println!(
        "  {} Saved to {}",
        style("→").dim(),
        style(relative_file).cyan()
    );
    if !plan.skipped.is_empty() {
        println!(
            "  {} Skipped (already exist): {}",
            style("*").dim(),
            plan.skipped.join(", ")
        );
    }
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
