use anyhow::{bail, Context, Result};
use console::style;
use std::path::Path;

use super::validate::validate_lanes;
use super::{FledgeFileWithLanes, LANES_PUBLISH_SCHEMA};
use crate::publish::PublishRequest;

pub(crate) fn publish_lanes(
    path: &Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let (token, path) = crate::publish::publish_preflight(path)?;

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

    let owner = crate::publish::resolve_owner(org, &token)?;

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

    let mut extra_fields = serde_json::Map::new();
    extra_fields.insert("lanes_published".to_string(), serde_json::json!(lane_names));
    extra_fields.insert(
        "import_hint".to_string(),
        serde_json::Value::from(format!("fledge lanes import {owner}/{repo_name}")),
    );

    let success_command = format!("fledge lanes import {}/{}", owner, repo_name);
    crate::publish::run_publish(PublishRequest {
        path: &path,
        owner: &owner,
        repo_name: &repo_name,
        description: desc,
        private,
        org,
        token: &token,
        yes,
        json,
        topic: "fledge-lane",
        commit_message: "Publish fledge lanes",
        noun: "lane",
        schema_version: LANES_PUBLISH_SCHEMA,
        success_verb: "Import",
        success_command: &success_command,
        extra_fields,
    })
}
