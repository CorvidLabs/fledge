use anyhow::{bail, Context, Result};
use console::style;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{load_registry, plugin_bin_dir, plugins_dir, PluginCapabilities, PluginManifest};

pub(super) fn run_plugin_cmd(name: &str, args: &[String]) -> Result<()> {
    let bin_path = super::resolve_plugin_command(name)
        .or_else(|| resolve_plugin_by_name(name))
        .ok_or_else(|| {
            let hint = match find_commands_for_plugin(name) {
                Some(cmds) if !cmds.is_empty() => format!(
                    "\n  Did you mean one of its commands? {}",
                    style(cmds.join(", ")).cyan()
                ),
                _ => String::new(),
            };
            anyhow::anyhow!(
                "Plugin command '{}' not found.{}\n  Run {} to see installed plugins.",
                name,
                hint,
                style("fledge plugin list").cyan()
            )
        })?;

    if let Some((plugin_name, plugin_version, plugin_dir, capabilities)) =
        resolve_protocol_info(name)
    {
        return crate::protocol::run_protocol_plugin(
            &bin_path,
            args,
            &plugin_name,
            &plugin_version,
            &plugin_dir,
            &capabilities,
        );
    }

    let mut cmd = Command::new(&bin_path);
    cmd.args(args);
    if let Some(plugin_dir) = resolve_plugin_source_dir(&bin_path) {
        cmd.env("FLEDGE_PLUGIN_DIR", &plugin_dir);
    }
    let status = cmd
        .status()
        .with_context(|| format!("running plugin '{name}'"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Plugin '{}' exited with code {}", name, code);
    }

    Ok(())
}

/// Compute the plugin's source directory from the resolved binary path.
///
/// `bin_path` is typically the symlink at `~/.config/fledge/plugins/bin/<cmd>`,
/// which resolves to `~/.config/fledge/plugins/<plugin>/bin/<cmd>` (or
/// similar). The plugin's source dir is two levels up from the resolved
/// binary — that's the location where multi-file shell plugins keep their
/// helpers, and what `FLEDGE_PLUGIN_DIR` should point to.
///
/// Returns `None` if the path can't be resolved (in which case we just don't
/// set the env var — plugins that don't rely on it work as before).
pub(super) fn resolve_plugin_source_dir(bin_path: &Path) -> Option<PathBuf> {
    let resolved = std::fs::canonicalize(bin_path).ok()?;
    // <plugin_dir>/<bin_subdir>/<binary> — take parent twice.
    resolved.parent()?.parent().map(|p| p.to_path_buf())
}

pub(super) fn run_hook(plugin_dir: &Path, hook: &str, event: &str) -> Result<()> {
    println!(
        "  {} Running {} hook...",
        style("▶️").cyan().bold(),
        style(event).dim()
    );

    let hook_path = plugin_dir.join(hook);
    let status = if hook_path.exists() {
        let canonical_hook = hook_path
            .canonicalize()
            .with_context(|| format!("canonicalizing hook path '{}'", hook))?;
        let canonical_plugin_dir = plugin_dir
            .canonicalize()
            .unwrap_or_else(|_| plugin_dir.to_path_buf());
        if !canonical_hook.starts_with(&canonical_plugin_dir) {
            bail!("Hook path '{}' escapes plugin directory", hook);
        }
        super::make_executable(&hook_path)?;
        Command::new(&hook_path)
            .current_dir(plugin_dir)
            .env("FLEDGE_PLUGIN_DIR", plugin_dir)
            .status()
            .with_context(|| format!("running {event} hook"))?
    } else {
        let parts: Vec<&str> = hook.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Empty hook command for {event}");
        }
        Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(plugin_dir)
            .env("FLEDGE_PLUGIN_DIR", plugin_dir)
            .status()
            .with_context(|| format!("running {event} hook"))?
    };

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Hook '{}' exited with code {}", event, code);
    }
    Ok(())
}

fn resolve_plugin_by_name(plugin_name: &str) -> Option<PathBuf> {
    let registry = load_registry().ok()?;
    let entry = registry
        .plugins
        .iter()
        .find(|p| p.name == plugin_name || p.name == format!("fledge-{plugin_name}"))?;
    let first_cmd = entry.commands.first()?;
    super::resolve_plugin_command(first_cmd)
}

fn find_commands_for_plugin(plugin_name: &str) -> Option<Vec<String>> {
    let registry = load_registry().ok()?;
    registry
        .plugins
        .iter()
        .find(|p| p.name == plugin_name || p.name == format!("fledge-{plugin_name}"))
        .map(|p| p.commands.clone())
}

fn resolve_protocol_info(name: &str) -> Option<(String, String, PathBuf, PluginCapabilities)> {
    let registry = load_registry().ok()?;
    let entry = registry.plugins.iter().find(|p| {
        p.name == name || p.name == format!("fledge-{name}") || p.commands.iter().any(|c| c == name)
    })?;

    let plugin_dir = plugins_dir().join(&entry.name);
    let manifest_path = plugin_dir.join("plugin.toml");
    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: PluginManifest = toml::from_str(&content).ok()?;

    let caps = entry
        .capabilities
        .clone()
        .unwrap_or_else(|| manifest.capabilities.clone());

    match &manifest.plugin.protocol {
        Some(proto) if proto == "fledge-v1" => Some((
            manifest.plugin.name,
            manifest.plugin.version,
            plugin_dir,
            caps,
        )),
        Some(proto) => {
            eprintln!(
                "{} Plugin '{}' requires protocol '{}' which is not supported.\n  Try updating fledge: {}",
                style("Error:").red().bold(),
                entry.name,
                proto,
                style("cargo install fledge").cyan()
            );
            None
        }
        None => None,
    }
}

pub(super) fn which_fledge_plugin(name: &str) -> Option<PathBuf> {
    let target = format!("fledge-{name}");
    let path_var = std::env::var("PATH").ok()?;

    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(&target);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}
