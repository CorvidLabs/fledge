use anyhow::{Context, Result};
use console::style;
use std::fs;

use super::{
    load_registry, plugin_bin_dir, plugins_dir, remove_plugin_path, run_hook, save_registry,
    PluginManifest, PLUGINS_REMOVE_SCHEMA,
};

pub(crate) fn remove_plugin(name: &str, json: bool) -> Result<()> {
    let mut registry = load_registry()?;
    let idx = registry
        .plugins
        .iter()
        .position(|p| p.name == name || p.name == format!("fledge-{name}"))
        .ok_or_else(|| {
            let installed: Vec<&str> = registry.plugins.iter().map(|p| p.name.as_str()).collect();
            if installed.is_empty() {
                anyhow::anyhow!("No plugins installed.")
            } else {
                anyhow::anyhow!(
                    "Plugin '{}' is not installed.\n  Installed: {}",
                    name,
                    installed.join(", ")
                )
            }
        })?;

    let entry = &registry.plugins[idx];
    let bin_dir = plugin_bin_dir();

    for cmd_name in &entry.commands {
        let link = bin_dir.join(format!("fledge-{cmd_name}"));
        fs::remove_file(&link).ok();
    }

    let plugin_dir = plugins_dir().join(&entry.name);

    // Read manifest before deleting so we can run the post_remove hook
    let post_remove_hook = plugin_dir
        .join("plugin.toml")
        .exists()
        .then(|| {
            fs::read_to_string(plugin_dir.join("plugin.toml"))
                .ok()
                .and_then(|s| toml::from_str::<PluginManifest>(&s).ok())
                .and_then(|m| m.hooks.post_remove)
        })
        .flatten();

    if let Some(ref hook) = post_remove_hook {
        run_hook(&plugin_dir, hook, "post_remove")?;
    }

    if plugin_dir.exists() {
        remove_plugin_path(&plugin_dir).context("removing plugin directory")?;
    }

    let removed_name = entry.name.clone();
    let removed_source = entry.source.clone();
    let removed_version = entry.version.clone();
    let removed_commands = entry.commands.clone();
    registry.plugins.remove(idx);
    save_registry(&registry)?;

    if json {
        let result = serde_json::json!({
            "schema_version": PLUGINS_REMOVE_SCHEMA,
            "action": "remove",
            "removed": {
                "name": removed_name,
                "source": removed_source,
                "version": removed_version,
                "commands": removed_commands,
            },
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} Removed {}",
            style("✅").green().bold(),
            style(&removed_name).green()
        );
    }

    Ok(())
}
