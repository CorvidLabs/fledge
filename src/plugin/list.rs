use anyhow::Result;
use console::style;
use std::fs;

use crate::trust::determine_trust_tier;

use super::{
    load_registry, plugins_dir, PluginManifest, PLUGINS_AUDIT_SCHEMA, PLUGINS_LIST_SCHEMA,
};

pub(crate) fn list_plugins(json: bool) -> Result<()> {
    let registry = load_registry()?;

    if registry.plugins.is_empty() {
        if json {
            let result = serde_json::json!({
                "schema_version": PLUGINS_LIST_SCHEMA,
                "plugins": [],
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "{} No plugins installed. Use {} to find plugins.",
                style("*").cyan().bold(),
                style("fledge plugin search").cyan()
            );
        }
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = registry
            .plugins
            .iter()
            .map(|p| {
                let tier = determine_trust_tier(&p.source);
                serde_json::json!({
                    "name": p.name,
                    "version": p.version,
                    "source": p.source,
                    "installed": p.installed,
                    "commands": p.commands,
                    "pinned_ref": p.pinned_ref,
                    "trust_tier": tier.label(),
                })
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": PLUGINS_LIST_SCHEMA,
            "plugins": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("{}", style("Installed plugins:").bold());
    let max_name = registry
        .plugins
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(0);

    for plugin in &registry.plugins {
        let tier = determine_trust_tier(&plugin.source);
        let version_str = match &plugin.pinned_ref {
            Some(r) => format!("v{} (pinned: {})", plugin.version, r),
            None => format!("v{}", plugin.version),
        };
        println!(
            "  {:<width$}  {}  [{}]  {}",
            style(&plugin.name).green(),
            style(&version_str).dim(),
            tier.styled_label(),
            style(format!("({})", plugin.source)).dim(),
            width = max_name,
        );
        if !plugin.commands.is_empty() {
            println!(
                "  {:<width$}  Commands: {}",
                "",
                style(plugin.commands.join(", ")).cyan(),
                width = max_name,
            );
        }
    }

    Ok(())
}

pub(crate) fn audit_plugins(json: bool) -> Result<()> {
    use crate::trust::TrustTier;

    let registry = load_registry()?;

    if registry.plugins.is_empty() {
        if json {
            let result = serde_json::json!({
                "schema_version": PLUGINS_AUDIT_SCHEMA,
                "audit": [],
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("{} No plugins installed.", style("*").cyan().bold());
        }
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = registry
            .plugins
            .iter()
            .map(|p| {
                let tier = determine_trust_tier(&p.source);
                let caps = p.capabilities.as_ref();
                serde_json::json!({
                    "name": p.name,
                    "version": p.version,
                    "source": p.source,
                    "trust_tier": tier.label(),
                    "capabilities": {
                        "exec": caps.is_some_and(|c| c.exec),
                        "store": caps.is_some_and(|c| c.store),
                        "metadata": caps.is_some_and(|c| c.metadata),
                    },
                    "commands": p.commands,
                    "has_lifecycle_hooks": has_lifecycle_hooks(&p.name),
                })
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": PLUGINS_AUDIT_SCHEMA,
            "audit": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("{}", style("Plugin Security Audit").bold());
    println!();

    for plugin in &registry.plugins {
        let tier = determine_trust_tier(&plugin.source);
        println!(
            "  {} {} v{} [{}]",
            style("•").dim(),
            style(&plugin.name).green(),
            plugin.version,
            tier.styled_label(),
        );
        println!("    Source: {}", style(&plugin.source).dim(),);

        let caps = plugin.capabilities.as_ref();
        let has_exec = caps.is_some_and(|c| c.exec);
        let has_store = caps.is_some_and(|c| c.store);
        let has_metadata = caps.is_some_and(|c| c.metadata);

        if has_exec || has_store || has_metadata {
            println!("    Capabilities:");
            if has_exec {
                println!(
                    "      {} exec — can run shell commands",
                    style("•").yellow()
                );
            }
            if has_store {
                println!(
                    "      {} store — can persist data between runs",
                    style("•").yellow()
                );
            }
            if has_metadata {
                println!(
                    "      {} metadata — can read project metadata and environment",
                    style("•").yellow()
                );
            }
        } else {
            println!("    Capabilities: {}", style("none").dim());
        }

        if has_lifecycle_hooks(&plugin.name) {
            let hooks = get_lifecycle_hooks(&plugin.name);
            if !hooks.is_empty() {
                println!("    Lifecycle hooks:");
                for (event, cmd) in &hooks {
                    println!(
                        "      {} {} → {}",
                        style("•").cyan(),
                        style(event).dim(),
                        style(cmd).dim()
                    );
                }
            }
        }

        if !plugin.commands.is_empty() {
            println!("    Commands: {}", style(plugin.commands.join(", ")).cyan());
        }

        if tier == TrustTier::Unverified && (has_exec || has_metadata) {
            println!(
                "    {} Unverified plugin with elevated capabilities",
                style("⚠").yellow().bold()
            );
        }

        println!();
    }

    let unverified_count = registry
        .plugins
        .iter()
        .filter(|p| determine_trust_tier(&p.source) == TrustTier::Unverified)
        .count();
    let elevated_count = registry
        .plugins
        .iter()
        .filter(|p| {
            let caps = p.capabilities.as_ref();
            caps.is_some_and(|c| c.exec || c.metadata)
        })
        .count();

    println!(
        "  {} {} plugin(s), {} unverified, {} with elevated capabilities",
        style("Summary:").bold(),
        registry.plugins.len(),
        unverified_count,
        elevated_count
    );

    Ok(())
}

pub(crate) fn has_lifecycle_hooks(plugin_name: &str) -> bool {
    !get_lifecycle_hooks(plugin_name).is_empty()
}

pub(crate) fn get_lifecycle_hooks(plugin_name: &str) -> Vec<(String, String)> {
    let plugin_dir = plugins_dir().join(plugin_name);
    let manifest_path = plugin_dir.join("plugin.toml");
    if !manifest_path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let manifest: PluginManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    let mut hooks = Vec::new();
    if let Some(ref h) = manifest.hooks.pre_init {
        hooks.push(("pre_init".to_string(), h.clone()));
    }
    if let Some(ref h) = manifest.hooks.post_work_start {
        hooks.push(("post_work_start".to_string(), h.clone()));
    }
    if let Some(ref h) = manifest.hooks.pre_pr {
        hooks.push(("pre_pr".to_string(), h.clone()));
    }
    if let Some(ref h) = manifest.hooks.post_install {
        hooks.push(("post_install".to_string(), h.clone()));
    }
    if let Some(ref h) = manifest.hooks.post_remove {
        hooks.push(("post_remove".to_string(), h.clone()));
    }
    hooks
}
