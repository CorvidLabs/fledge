use anyhow::{Context, Result};
use console::style;
use std::fs;
use std::path::Path;

use super::{validate_plugin, PluginManifest, PLUGINS_PUBLISH_SCHEMA};

pub(crate) fn publish_plugin(
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

    let manifest_path = path.join("plugin.toml");
    validate_plugin(&path, false, false)?;

    let content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let manifest: PluginManifest = toml::from_str(&content).context("Invalid plugin.toml")?;

    let repo_name = &manifest.plugin.name;
    let desc = description
        .or(manifest.plugin.description.as_deref())
        .unwrap_or("A fledge plugin");

    let owner = match org {
        Some(o) => o.to_string(),
        None => crate::publish::get_authenticated_user(&token)?,
    };

    if !json {
        println!(
            "{} Publishing plugin {} as {}/{}",
            style("➡️").cyan().bold(),
            style(path.display()).dim(),
            style(&owner).green(),
            style(repo_name).green()
        );
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Checking repository:"))
    };
    let repo_exists = crate::publish::check_repo_exists(&owner, repo_name, &token)?;
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
                        "schema_version": PLUGINS_PUBLISH_SCHEMA,
                        "action": "publish",
                        "cancelled": true,
                        "repo": {
                            "owner": owner,
                            "name": repo_name,
                            "url": format!("https://github.com/{owner}/{repo_name}"),
                            "created": false,
                            "private": private,
                        },
                        "plugin": {
                            "name": manifest.plugin.name,
                            "version": manifest.plugin.version,
                            "description": desc,
                        },
                        "topic": "fledge-plugin",
                        "install_hint": format!("fledge plugins install {owner}/{repo_name}"),
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
        crate::publish::create_github_repo(repo_name, desc, private, org, &token)?;
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
    crate::publish::set_repo_topic(&owner, repo_name, "fledge-plugin", &token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style("fledge-plugin").cyan()
        );
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Pushing plugin files:"))
    };
    crate::publish::push_directory(&path, &owner, repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    if json {
        let result = serde_json::json!({
            "schema_version": PLUGINS_PUBLISH_SCHEMA,
            "action": "publish",
            "cancelled": false,
            "repo": {
                "owner": owner,
                "name": repo_name,
                "url": format!("https://github.com/{owner}/{repo_name}"),
                "created": created_repo,
                "private": private,
            },
            "plugin": {
                "name": manifest.plugin.name,
                "version": manifest.plugin.version,
                "description": desc,
            },
            "topic": "fledge-plugin",
            "install_hint": format!("fledge plugins install {owner}/{repo_name}"),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("  {} Pushed plugin files", style("✅").green().bold());
        println!(
            "\n{} Published! Install with:\n\n  {}",
            style("✅").green().bold(),
            style(format!("fledge plugins install {}/{}", owner, repo_name)).cyan()
        );
    }

    Ok(())
}
