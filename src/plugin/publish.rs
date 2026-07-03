use anyhow::{Context, Result};
use console::style;
use std::fs;
use std::path::Path;

use super::{validate_plugin, PluginManifest, PLUGINS_PUBLISH_SCHEMA};
use crate::publish::PublishRequest;

pub(crate) fn publish_plugin(
    path: &Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let (token, path) = crate::publish::publish_preflight(path)?;

    let manifest_path = path.join("plugin.toml");
    validate_plugin(&path, false, false)?;

    let content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let manifest: PluginManifest = toml::from_str(&content).context("Invalid plugin.toml")?;

    let repo_name = manifest.plugin.name.clone();
    let desc = description
        .or(manifest.plugin.description.as_deref())
        .unwrap_or("A fledge plugin");

    let owner = crate::publish::resolve_owner(org, &token)?;

    if !json {
        println!(
            "{} Publishing plugin {} as {}/{}",
            style("➡️").cyan().bold(),
            style(path.display()).dim(),
            style(&owner).green(),
            style(&repo_name).green()
        );
    }

    let mut extra_fields = serde_json::Map::new();
    extra_fields.insert(
        "plugin".to_string(),
        serde_json::json!({
            "name": manifest.plugin.name.clone(),
            "version": manifest.plugin.version.clone(),
            "description": desc,
        }),
    );
    extra_fields.insert(
        "install_hint".to_string(),
        serde_json::Value::from(format!("fledge plugins install {owner}/{repo_name}")),
    );

    let success_command = format!("fledge plugins install {}/{}", owner, repo_name);
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
        topic: "fledge-plugin",
        commit_message: "Publish fledge plugin",
        noun: "plugin",
        schema_version: PLUGINS_PUBLISH_SCHEMA,
        success_verb: "Install",
        success_command: &success_command,
        extra_fields,
    })
}
