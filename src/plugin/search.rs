use anyhow::{Context, Result};
use console::style;

use super::PLUGINS_SEARCH_SCHEMA;

pub(crate) fn search_plugins(
    query: Option<&str>,
    author: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<()> {
    let sp = crate::spinner::Spinner::start("Searching GitHub for plugins:");

    let config = crate::config::Config::load().ok();
    let token = config.as_ref().and_then(|c| c.github_token());

    let query_str = crate::search::build_search_query_ex(query, author, "fledge-plugin");
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

    let items = body["items"].as_array().unwrap_or(&Vec::new()).clone();

    if items.is_empty() {
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
        let entries: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                let owner = item["owner"]["login"].as_str().unwrap_or("");
                let tier = crate::trust::determine_trust_tier_from_owner(owner);
                serde_json::json!({
                    "name": item["name"],
                    "full_name": item["full_name"],
                    "description": item["description"],
                    "stars": item["stargazers_count"],
                    "url": item["html_url"],
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

    println!("{}", style("Available plugins:").bold());
    let max_name = items
        .iter()
        .filter_map(|i| i["full_name"].as_str())
        .map(|n| n.len())
        .max()
        .unwrap_or(0);

    for item in &items {
        let full_name = item["full_name"].as_str().unwrap_or("?");
        let owner = item["owner"]["login"].as_str().unwrap_or("");
        let tier = crate::trust::determine_trust_tier_from_owner(owner);
        let desc = item["description"].as_str().unwrap_or("(no description)");
        let stars = item["stargazers_count"].as_u64().unwrap_or(0);
        println!(
            "  {:<width$}  [{}]  {}  {}",
            style(full_name).green(),
            tier.styled_label(),
            style(desc).dim(),
            style(format!("⭐ {stars}")).yellow(),
            width = max_name,
        );
    }

    println!(
        "\n  Install with: {}",
        style("fledge plugin install <owner/repo>").cyan()
    );

    Ok(())
}
