use anyhow::{Context, Result};
use console::style;

use super::PLUGINS_SEARCH_SCHEMA;
use crate::trust::TrustTier;

pub(crate) fn search_plugins(
    query: Option<&str>,
    author: Option<&str>,
    topic: Option<&str>,
    trust_tier: Option<TrustTier>,
    limit: usize,
    interactive: bool,
    json: bool,
) -> Result<()> {
    let sp = crate::spinner::Spinner::start("Searching GitHub for plugins:");

    let config = crate::config::Config::load().ok();
    let token = config.as_ref().and_then(|c| c.github_token());

    let query_str = crate::search::build_search_query(query, author, "fledge-plugin", topic);
    let limit_str = limit.to_string();
    let body = crate::github::github_api_get(
        "/search/repositories",
        token.as_deref(),
        &[
            ("q", &query_str),
            ("sort", "stars"),
            ("per_page", &limit_str),
        ],
    )
    .context("searching GitHub for plugins")?;

    sp.finish();

    let results = crate::search::parse_search_response(&body)?;
    // GitHub's API has no concept of fledge's trust tiers, so the filter is
    // applied client-side after parsing. Each result's tier is computed from
    // the owner via the same classifier `plugins list/audit` use.
    let results: Vec<crate::search::SearchResult> = match trust_tier {
        Some(want) => results
            .into_iter()
            .filter(|r| crate::trust::determine_trust_tier_from_owner(&r.owner) == want)
            .collect(),
        None => results,
    };

    if results.is_empty() {
        if json {
            let result = serde_json::json!({
                "schema_version": PLUGINS_SEARCH_SCHEMA,
                "results": [],
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "{} No plugins found{}.",
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
                let tier = crate::trust::determine_trust_tier_from_owner(&r.owner);
                serde_json::json!({
                    "name": r.name,
                    "full_name": r.full_name(),
                    "description": r.description,
                    "stars": r.stars,
                    "url": r.url,
                    "topics": r.topics,
                    "trust_tier": tier.label(),
                })
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": PLUGINS_SEARCH_SCHEMA,
            "results": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if interactive {
        return interactive_search(&results);
    }

    print_results(&results);
    Ok(())
}

fn print_results(results: &[crate::search::SearchResult]) {
    println!("{}", style("Available plugins:").bold());
    let max_name = results
        .iter()
        .map(|r| r.full_name().len())
        .max()
        .unwrap_or(0);

    for r in results {
        let tier = crate::trust::determine_trust_tier_from_owner(&r.owner);
        let topics_str = if r.topics.is_empty() {
            String::new()
        } else {
            let filtered: Vec<&str> = r
                .topics
                .iter()
                .filter(|t| *t != "fledge-plugin")
                .map(|t| t.as_str())
                .collect();
            if filtered.is_empty() {
                String::new()
            } else {
                format!(" {}", style(filtered.join(", ")).cyan())
            }
        };
        println!(
            "  {:<width$}  [{}]  {}  {}{}",
            style(r.full_name()).green(),
            tier.styled_label(),
            style(&r.description).dim(),
            style(format!("⭐ {}", r.stars)).yellow(),
            topics_str,
            width = max_name,
        );
    }

    println!(
        "\n  Install with: {}",
        style("fledge plugins install <owner/repo>").cyan()
    );
    println!(
        "  Or use:       {}",
        style("fledge plugins search --interactive").cyan()
    );
}

fn interactive_search(results: &[crate::search::SearchResult]) -> Result<()> {
    crate::utils::require_interactive("--interactive")?;

    let term_width = console::Term::stdout().size().1 as usize;
    let max_item_width = term_width.saturating_sub(4);

    let items: Vec<String> = results
        .iter()
        .map(|r| {
            let tier = crate::trust::determine_trust_tier_from_owner(&r.owner);
            let line = format!(
                "{:<40} [{}]  {}",
                r.full_name(),
                tier.label(),
                r.description
            );
            if line.len() > max_item_width {
                format!("{}…", &line[..max_item_width - 1])
            } else {
                line
            }
        })
        .collect();

    let selection = dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Select a plugin to install")
        .items(&items)
        .default(0)
        .max_length(15)
        .highlight_matches(true)
        .interact_opt()?;

    let Some(idx) = selection else {
        println!("Cancelled.");
        return Ok(());
    };

    let chosen = &results[idx];
    println!(
        "\n  Installing {} ...\n",
        style(chosen.full_name()).green().bold()
    );

    super::install::install_action(Some(&chosen.full_name()), false, false, false, false)
}
