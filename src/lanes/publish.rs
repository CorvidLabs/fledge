use anyhow::{bail, Context, Result};
use console::style;
use std::path::Path;

use super::validate::validate_lanes;
use super::{FledgeFileWithLanes, LANES_PUBLISH_SCHEMA};

pub(crate) fn publish_lanes(
    path: &Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let yes = yes || crate::utils::is_non_interactive() || json;
    let config = crate::config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;

    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;

    let fledge_toml = path.join("fledge.toml");
    if !fledge_toml.exists() {
        bail!(
            "No fledge.toml found in {}. Lanes must be defined in a fledge.toml file.",
            path.display()
        );
    }

    validate_lanes(&path, false, false)?;

    let content = std::fs::read_to_string(&fledge_toml).context("reading fledge.toml")?;
    let parsed: FledgeFileWithLanes = toml::from_str(&content).context("parsing fledge.toml")?;

    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fledge-lanes");
    let repo_name = dir_name.to_string();
    let desc = description.unwrap_or("Shared fledge lanes");

    let owner = match org {
        Some(o) => o.to_string(),
        None => crate::publish::get_authenticated_user(&token)?,
    };

    let lane_names: Vec<String> = parsed.lanes.keys().cloned().collect();
    if !json {
        println!(
            "{} Publishing {} lanes as {}/{}",
            style("➡️").cyan().bold(),
            style(lane_names.len()).green(),
            style(&owner).green(),
            style(&repo_name).green()
        );
        println!("  Lanes: {}", style(lane_names.join(", ")).dim());
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Checking repository:"))
    };
    let repo_exists = crate::publish::check_repo_exists(&owner, &repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    let mut created_repo = false;
    if repo_exists {
        if !yes {
            crate::utils::require_interactive("yes")?;
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(format!(
                        "Repository {}/{} already exists. Push update?",
                        owner, repo_name
                    ))
                    .default(false)
                    .interact()?;

            if !confirm {
                if json {
                    let result = serde_json::json!({
                        "schema_version": LANES_PUBLISH_SCHEMA,
                        "action": "publish",
                        "cancelled": true,
                        "repo": {
                            "owner": owner,
                            "name": repo_name,
                            "url": format!("https://github.com/{owner}/{repo_name}"),
                            "created": false,
                            "private": private,
                        },
                        "lanes_published": lane_names,
                        "topic": "fledge-lane",
                        "import_hint": format!("fledge lanes import {owner}/{repo_name}"),
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{} Cancelled.", style("*").cyan().bold());
                }
                return Ok(());
            }
        }
    } else {
        let sp = if json {
            None
        } else {
            Some(crate::spinner::Spinner::start("Creating repository:"))
        };
        crate::publish::create_github_repo(&repo_name, desc, private, org, &token)?;
        if let Some(s) = sp {
            s.finish();
        }
        created_repo = true;
        if !json {
            println!(
                "  {} Created repository {}/{}",
                style("✅").green().bold(),
                owner,
                repo_name
            );
        }
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Setting repository topics:"))
    };
    crate::publish::set_repo_topic(&owner, &repo_name, "fledge-lane", &token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style("fledge-lane").cyan()
        );
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Pushing lane files:"))
    };
    crate::publish::push_directory(&path, &owner, &repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    if json {
        let result = serde_json::json!({
            "schema_version": LANES_PUBLISH_SCHEMA,
            "action": "publish",
            "cancelled": false,
            "repo": {
                "owner": owner,
                "name": repo_name,
                "url": format!("https://github.com/{owner}/{repo_name}"),
                "created": created_repo,
                "private": private,
            },
            "lanes_published": lane_names,
            "topic": "fledge-lane",
            "import_hint": format!("fledge lanes import {owner}/{repo_name}"),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("  {} Pushed lane files", style("✅").green().bold());
        println!(
            "\n{} Published! Import with:\n\n  {}",
            style("✅").green().bold(),
            style(format!("fledge lanes import {}/{}", owner, repo_name)).cyan()
        );
    }

    Ok(())
}
