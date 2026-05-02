use anyhow::Result;
use console::style;

use crate::config;
use crate::create_template;
use crate::github;
use crate::init;
use crate::publish;
use crate::search;
use crate::spinner;
use crate::templates;
use crate::trust;
use crate::utils;
use crate::validate;
use crate::TemplatesSubcommand;

pub fn handle_templates(action: TemplatesSubcommand) -> Result<()> {
    match action {
        TemplatesSubcommand::Init {
            name,
            template,
            output,
            author,
            org,
            no_git,
            no_install,
            refresh,
            dry_run,
            yes,
            trust_hooks,
            json,
        } => {
            init::run(init::InitOptions {
                name,
                template,
                output,
                author,
                org,
                no_git,
                no_install,
                refresh,
                dry_run,
                yes,
                trust_hooks,
                json,
            })?;
        }
        TemplatesSubcommand::Create {
            name,
            output,
            description,
            render_patterns,
            hooks,
            prompts,
            yes,
            json,
        } => {
            create_template::run(create_template::CreateTemplateOptions {
                name,
                output,
                description,
                render_patterns,
                hooks,
                prompts,
                yes,
                json,
            })?;
        }
        TemplatesSubcommand::Validate { path, strict, json } => {
            validate::run(validate::ValidateOptions { path, strict, json })?;
        }
        TemplatesSubcommand::List { json } => {
            list_templates(json)?;
        }
        TemplatesSubcommand::Search {
            query,
            author,
            limit,
            json,
        } => {
            search_templates(query.as_deref(), author.as_deref(), limit, json)?;
        }
        TemplatesSubcommand::Publish {
            path,
            org,
            private,
            description,
            yes,
            json,
        } => {
            publish_template(
                &path,
                org.as_deref(),
                private,
                description.as_deref(),
                yes,
                json,
            )?;
        }
    }
    Ok(())
}

pub fn install_completions(shell: Option<clap_complete::Shell>) -> Result<()> {
    use crate::cli::Cli;
    use clap::CommandFactory;

    let shell = shell.unwrap_or_else(|| {
        let shell_env = std::env::var("SHELL").unwrap_or_default();
        if shell_env.ends_with("zsh") {
            clap_complete::Shell::Zsh
        } else if shell_env.ends_with("fish") {
            clap_complete::Shell::Fish
        } else {
            clap_complete::Shell::Bash
        }
    });

    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;

    let dest = match shell {
        clap_complete::Shell::Bash => {
            let dir = home.join(".local/share/bash-completion/completions");
            std::fs::create_dir_all(&dir)?;
            dir.join("fledge")
        }
        clap_complete::Shell::Zsh => {
            let dir = home.join(".zfunc");
            std::fs::create_dir_all(&dir)?;
            dir.join("_fledge")
        }
        clap_complete::Shell::Fish => {
            let dir = home.join(".config/fish/completions");
            std::fs::create_dir_all(&dir)?;
            dir.join("fledge.fish")
        }
        _ => anyhow::bail!(
            "auto-install not supported for {:?} — use `fledge completions <shell>` to generate manually",
            shell
        ),
    };

    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "fledge", &mut buf);
    std::fs::write(&dest, buf)?;

    println!(
        "{} Installed {} completions to {}",
        style("✅").green().bold(),
        style(format!("{shell:?}")).cyan(),
        style(dest.display()).dim()
    );

    if matches!(shell, clap_complete::Shell::Zsh) {
        println!(
            "\n  {}",
            style("Add to your .zshrc if not already present:").dim()
        );
        println!("    fpath=(~/.zfunc $fpath)");
        println!("    autoload -Uz compinit && compinit");
    }

    Ok(())
}

pub fn list_templates(json: bool) -> Result<()> {
    let config = config::Config::load()?;
    let extra_paths = config.extra_template_paths();
    let token = config.github_token();
    let available = templates::discover_templates_with_repos(
        &extra_paths,
        config.template_repos(),
        token.as_deref(),
    )?;

    let hint = "Configure template sources via `fledge config add templates.repos <owner/repo>`, add templates to the templates/ directory, or set templates.paths via `fledge config add templates.paths <path>`.";

    if json {
        let entries: Vec<serde_json::Value> = available
            .iter()
            .map(|t| {
                let source_kind = match &t.source {
                    Some(s) if s.starts_with("http") || s.contains('/') => "remote",
                    Some(_) => "local",
                    None => "builtin",
                };
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "source": source_kind,
                    "source_ref": t.source,
                    "path": t.path.display().to_string(),
                })
            })
            .collect();
        let mut result = serde_json::json!({
            "schema_version": templates::TEMPLATES_LIST_SCHEMA,
            "templates": entries,
        });
        if available.is_empty() {
            result["hint"] = serde_json::Value::String(hint.to_string());
        }
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if available.is_empty() {
        println!(
            "{} No templates configured.\n\n  {}",
            style("*").cyan().bold(),
            style(hint).dim()
        );
    } else {
        println!("{}", style("Available templates:").bold());
        for t in &available {
            println!(
                "  {:<14} {}",
                style(&t.name).green(),
                style(&t.description).dim()
            );
        }
    }

    Ok(())
}

pub fn search_templates(
    query: Option<&str>,
    author: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<()> {
    use anyhow::Context as _;
    let config = config::Config::load()?;
    let token = config.github_token();
    let q = search::build_search_query_ex(query, author, "fledge-template");
    let per_page = limit.clamp(1, 100).to_string();

    let sp = spinner::Spinner::start("Searching GitHub for community templates:");
    let body = github::github_api_get(
        "/search/repositories",
        token.as_deref(),
        &[("q", &q), ("sort", "stars"), ("per_page", &per_page)],
    )
    .context("searching GitHub for template repos")?;
    sp.finish();

    let mut results = search::parse_search_response(&body)?;
    results.truncate(limit);

    if results.is_empty() {
        if json {
            let result = serde_json::json!({
                "schema_version": templates::TEMPLATES_SEARCH_SCHEMA,
                "results": [],
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "{} No community templates found{}.",
                style("*").cyan().bold(),
                query
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
                let tier = trust::determine_trust_tier_from_owner(&r.owner);
                serde_json::json!({
                    "owner": r.owner,
                    "name": r.name,
                    "description": r.description,
                    "stars": r.stars,
                    "url": r.url,
                    "topics": r.topics,
                    "trust_tier": tier.label(),
                })
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": templates::TEMPLATES_SEARCH_SCHEMA,
            "results": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("{}\n", style("Community templates on GitHub:").bold());
    let max_name = results
        .iter()
        .map(|r| r.full_name().len())
        .max()
        .unwrap_or(0);
    for r in &results {
        let tier = trust::determine_trust_tier_from_owner(&r.owner);
        let stars = search::format_stars(r.stars);
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
        style("Use with: fledge templates init --template <owner/repo>").dim()
    );
    Ok(())
}

pub fn publish_template(
    path: &std::path::Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    use anyhow::Context as _;
    let yes = yes || utils::is_non_interactive() || json;
    let config = config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;

    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;

    // Validate the template before publishing — same gate `fledge templates validate` uses.
    validate::run(validate::ValidateOptions {
        path: path.clone(),
        strict: false,
        json: false,
    })?;

    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fledge-template");
    let repo_name = dir_name.to_string();
    let desc = description.unwrap_or("A fledge template");

    let owner = match org {
        Some(o) => o.to_string(),
        None => publish::get_authenticated_user(&token)?,
    };

    if !json {
        println!(
            "{} Publishing template as {}/{}",
            style("➡️").cyan().bold(),
            style(&owner).green(),
            style(&repo_name).green()
        );
    }

    let sp = if json {
        None
    } else {
        Some(spinner::Spinner::start("Checking repository:"))
    };
    let repo_exists = publish::check_repo_exists(&owner, &repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    let mut created_repo = false;
    if repo_exists {
        if !yes {
            utils::require_interactive("yes")?;
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
                        "schema_version": templates::TEMPLATES_PUBLISH_SCHEMA,
                        "action": "publish",
                        "cancelled": true,
                        "repo": {
                            "owner": owner,
                            "name": repo_name,
                            "url": format!("https://github.com/{owner}/{repo_name}"),
                            "created": false,
                            "private": private,
                        },
                        "template": {
                            "description": desc,
                        },
                        "topic": "fledge-template",
                        "use_hint": format!("fledge templates init <name> --template {owner}/{repo_name}"),
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
            Some(spinner::Spinner::start("Creating repository:"))
        };
        publish::create_github_repo(&repo_name, desc, private, org, &token)?;
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
        Some(spinner::Spinner::start("Setting repository topics:"))
    };
    publish::set_repo_topic(&owner, &repo_name, "fledge-template", &token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style("fledge-template").cyan()
        );
    }

    let sp = if json {
        None
    } else {
        Some(spinner::Spinner::start("Pushing template files:"))
    };
    publish::push_directory(&path, &owner, &repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    if json {
        let result = serde_json::json!({
            "schema_version": templates::TEMPLATES_PUBLISH_SCHEMA,
            "action": "publish",
            "cancelled": false,
            "repo": {
                "owner": owner,
                "name": repo_name,
                "url": format!("https://github.com/{owner}/{repo_name}"),
                "created": created_repo,
                "private": private,
            },
            "template": {
                "description": desc,
            },
            "topic": "fledge-template",
            "use_hint": format!("fledge templates init <name> --template {owner}/{repo_name}"),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("  {} Pushed template files", style("✅").green().bold());
        println!(
            "\n{} Published! Use with:\n\n  {}",
            style("✅").green().bold(),
            style(format!(
                "fledge templates init --template {}/{}",
                owner, repo_name
            ))
            .cyan()
        );
    }

    Ok(())
}
