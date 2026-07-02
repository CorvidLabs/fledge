use anyhow::{Context, Result};
use console::style;

use super::PLUGINS_RECOMMEND_SCHEMA;

struct Recommendation {
    repo: &'static str,
    reason: &'static str,
}

fn recommendations_for_language(lang: &str) -> Vec<Recommendation> {
    let mut recs = vec![
        Recommendation {
            repo: "CorvidLabs/fledge-plugin-github",
            reason: "PR and issue workflows",
        },
        Recommendation {
            repo: "CorvidLabs/fledge-plugin-deps",
            reason: "dependency checking",
        },
        Recommendation {
            repo: "CorvidLabs/fledge-plugin-metrics",
            reason: "project code metrics",
        },
        Recommendation {
            repo: "CorvidLabs/fledge-plugin-todo",
            reason: "TODO/FIXME tracking",
        },
        Recommendation {
            repo: "CorvidLabs/fledge-plugin-secrets",
            reason: "secret leak detection",
        },
        Recommendation {
            repo: "CorvidLabs/fledge-plugin-gitleaks",
            reason: "git history secret scanning",
        },
    ];

    match lang {
        "rust" => {
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-bench",
                reason: "Rust benchmarking",
            });
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-coverage",
                reason: "code coverage reporting",
            });
        }
        "node" => {
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-coverage",
                reason: "code coverage reporting",
            });
        }
        "python" => {
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-coverage",
                reason: "code coverage reporting",
            });
        }
        "go" => {
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-bench",
                reason: "benchmarking",
            });
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-coverage",
                reason: "code coverage reporting",
            });
        }
        "swift" => {
            recs.push(Recommendation {
                repo: "CorvidLabs/fledge-plugin-coverage",
                reason: "code coverage reporting",
            });
        }
        _ => {}
    }

    if std::path::Path::new("Dockerfile").exists()
        || std::path::Path::new("docker-compose.yml").exists()
    {
        recs.push(Recommendation {
            repo: "CorvidLabs/fledge-plugin-docker",
            reason: "Docker image management",
        });
    }

    if std::path::Path::new(".github").exists() {
        recs.push(Recommendation {
            repo: "CorvidLabs/fledge-plugin-github",
            reason: "GitHub Actions integration",
        });
    }

    // Dedup by repo: the base list already recommends fledge-plugin-github, and
    // the `.github` branch above can add it a second time. Keep the first entry.
    let mut seen = std::collections::HashSet::new();
    recs.retain(|r| seen.insert(r.repo));

    recs
}

pub(crate) fn recommend_plugins(json: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    let lang = crate::run::detect_project_type(&cwd);

    let registry =
        super::load_registry().unwrap_or_else(|_| super::PluginsRegistry { plugins: vec![] });
    // Match on the registry `source`, normalized so a shorthand recommendation
    // repo ("CorvidLabs/fledge-plugin-github") and an install recorded as a full
    // URL ("https://github.com/CorvidLabs/fledge-plugin-github.git") compare
    // equal. The old code compared a prefix-stripped "github" against the registry
    // *name* ("fledge-plugin-github"), which never matched, so installed plugins
    // were always re-recommended. HashSet keeps the lookup O(1).
    let installed_count = registry.plugins.len();
    let installed_sources: std::collections::HashSet<String> = registry
        .plugins
        .iter()
        .map(|p| super::normalize_source(&p.source))
        .collect();

    let recs = recommendations_for_language(lang);
    let new_recs: Vec<&Recommendation> = recs
        .iter()
        .filter(|r| !installed_sources.contains(&super::normalize_source(r.repo)))
        .collect();

    if json {
        let entries: Vec<serde_json::Value> = new_recs
            .iter()
            .map(|r| {
                serde_json::json!({
                    "repo": r.repo,
                    "reason": r.reason,
                })
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": PLUGINS_RECOMMEND_SCHEMA,
            "action": "plugins_recommend",
            "language": lang,
            "installed_count": installed_count,
            "recommendations": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!(
        "{} Detected project type: {}\n",
        style("*").cyan().bold(),
        style(lang).green().bold()
    );

    if new_recs.is_empty() {
        println!(
            "  {} All recommended plugins are already installed!",
            style("✓").green()
        );
        return Ok(());
    }

    println!("{}", style("Recommended plugins:").bold());
    let max_name = new_recs.iter().map(|r| r.repo.len()).max().unwrap_or(0);
    for r in &new_recs {
        println!(
            "  {:<width$}  {}",
            style(r.repo).green(),
            style(r.reason).dim(),
            width = max_name,
        );
    }

    if !crate::utils::is_interactive() {
        println!(
            "\n  Install with: {}",
            style("fledge plugins install <owner/repo>").cyan()
        );
        return Ok(());
    }

    println!();
    let install_all = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(format!(
            "Install all {} recommended plugins?",
            new_recs.len()
        ))
        .default(true)
        .interact()?;

    if !install_all {
        println!(
            "\n  Install individually: {}",
            style("fledge plugins install <owner/repo>").cyan()
        );
        return Ok(());
    }

    println!();
    for r in &new_recs {
        if let Err(e) = super::install::install_action(Some(r.repo), false, false, false, false) {
            eprintln!("  {} Failed to install {}: {}", style("✗").red(), r.repo, e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn recommendations_have_no_duplicate_repos() {
        // The base list recommends fledge-plugin-github, and a `.github` dir adds
        // it again; dedup must collapse them. Checked across every language arm.
        for lang in ["rust", "node", "python", "go", "swift", "unknown"] {
            let recs = recommendations_for_language(lang);
            let mut seen = HashSet::new();
            for r in &recs {
                assert!(
                    seen.insert(r.repo),
                    "duplicate repo {} in recommendations for {}",
                    r.repo,
                    lang
                );
            }
        }
    }
}
