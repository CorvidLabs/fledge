use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::{
    apply_git_auth, link_commands, load_registry, normalize_source, plugin_bin_dir, plugins_dir,
    run_build, save_registry, PluginManifest, PLUGINS_UPDATE_SCHEMA,
};
use crate::trust::parse_source_ref;

pub(crate) fn update_plugins(name: Option<&str>, defaults: bool, json: bool) -> Result<()> {
    use super::DEFAULT_PLUGINS;

    if defaults && name.is_some() {
        bail!("--defaults updates the curated set; do not pass a plugin name alongside it.");
    }
    let registry = load_registry()?;

    let targets: Vec<_> = if defaults {
        // Match each installed plugin's source against the DEFAULT_PLUGINS
        // list. Stored sources use either the shorthand `owner/repo` form
        // (the install-time input) or the normalized URL — accept both.
        // DEFAULT_PLUGINS entries may carry an `@ref` suffix; strip it
        // before comparing so pinned defaults still match stored sources.
        let is_default = |source: &str| -> bool {
            DEFAULT_PLUGINS.iter().any(|d| {
                let (base, _) = parse_source_ref(d);
                source == base
                    || source == *d
                    || source == normalize_source(d)
                    || source.trim_end_matches(".git").ends_with(base)
            })
        };
        let matched: Vec<_> = registry
            .plugins
            .iter()
            .filter(|p| is_default(&p.source))
            .collect();
        if matched.is_empty() {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": PLUGINS_UPDATE_SCHEMA,
                        "action": "update",
                        "scope": "defaults",
                        "results": [],
                        "summary": { "total": 0, "updated": 0, "skipped": 0, "failed": 0 },
                    }))?
                );
            } else {
                println!(
                    "{} No default plugins are installed. Run {} first.",
                    style("*").cyan().bold(),
                    style("fledge plugins install --defaults").cyan()
                );
            }
            return Ok(());
        }
        if !json {
            println!(
                "{} Updating {} of {} default plugins...",
                style("*").cyan().bold(),
                matched.len(),
                DEFAULT_PLUGINS.len()
            );
        }
        matched
    } else {
        match name {
            Some(n) => {
                let entry = registry
                    .plugins
                    .iter()
                    .find(|p| p.name == n || p.name == format!("fledge-{n}"))
                    .ok_or_else(|| anyhow::anyhow!("Plugin '{n}' is not installed."))?;
                vec![entry]
            }
            None => {
                if registry.plugins.is_empty() {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "schema_version": PLUGINS_UPDATE_SCHEMA,
                                "action": "update",
                                "scope": "all",
                                "results": [],
                                "summary": { "total": 0, "updated": 0, "skipped": 0, "failed": 0 },
                            }))?
                        );
                    } else {
                        println!("{} No plugins installed.", style("*").cyan().bold());
                    }
                    return Ok(());
                }
                registry.plugins.iter().collect()
            }
        }
    };

    // Collect results in JSON mode for a single structured output at the end.
    // Each entry has `name`, `status` ("updated" | "skipped" | "failed"),
    // and a free-form `detail` (e.g. version bumped to, or error reason).
    let mut results: Vec<serde_json::Value> = Vec::new();

    for entry in &targets {
        let plugin_dir = plugins_dir().join(&entry.name);
        if !plugin_dir.exists() {
            if !json {
                println!(
                    "  {} {} — directory missing, reinstall with {}",
                    style("⚠️").yellow(),
                    style(&entry.name).yellow(),
                    style(format!("fledge plugin install {} --force", entry.source)).cyan()
                );
            }
            results.push(serde_json::json!({
                "name": entry.name,
                "status": "failed",
                "detail": "directory missing — reinstall required",
            }));
            continue;
        }

        // Workspace-managed plugins (e.g. Merlin's bundled plugins under
        // `plugins/`) have no `.git` and are rebuilt by the host project's
        // init step. Skipping them silently keeps `plugins update` quiet
        // instead of warning about a git pull that was never going to work.
        // (Issue #382)
        if !is_git_repo(&plugin_dir) {
            if !json {
                println!(
                    "  {} {} — workspace-managed (no .git), skipped",
                    style("*").cyan().bold(),
                    style(&entry.name).cyan()
                );
            }
            results.push(serde_json::json!({
                "name": entry.name,
                "status": "skipped",
                "detail": "workspace-managed plugin (no .git in install dir) — managed by host project's init step",
            }));
            continue;
        }

        if let Some(ref pinned) = entry.pinned_ref {
            let latest = find_latest_tag(&plugin_dir);
            match latest {
                Some(ref tag) if tag != pinned => {
                    if defaults {
                        let sp = if json {
                            None
                        } else {
                            Some(crate::spinner::Spinner::start(&format!(
                                "Upgrading {} {} → {}:",
                                &entry.name, pinned, tag
                            )))
                        };

                        let checkout = Command::new("git")
                            .args(["checkout", tag])
                            .current_dir(&plugin_dir)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::piped())
                            .status()
                            .with_context(|| format!("checking out {} for {}", tag, entry.name))?;

                        if let Some(s) = sp {
                            s.finish();
                        }

                        if !checkout.success() {
                            if !json {
                                println!(
                                    "  {} {} — failed to checkout {}, try:\n    {}",
                                    style("⚠️").yellow(),
                                    style(&entry.name).yellow(),
                                    style(tag).dim(),
                                    style(format!(
                                        "fledge plugin install {}@{} --force",
                                        entry.source, tag
                                    ))
                                    .cyan()
                                );
                            }
                            results.push(serde_json::json!({
                                "name": entry.name,
                                "status": "failed",
                                "detail": format!("git checkout {tag} failed — reinstall required"),
                            }));
                            continue;
                        }

                        let result = rebuild_after_fetch(entry, &plugin_dir, Some(tag), json)?;
                        results.push(result);
                    } else {
                        if !json {
                            println!(
                                "  {} {} — pinned to {}, latest tag is {}. To upgrade:\n    {}",
                                style("*").cyan().bold(),
                                style(&entry.name).cyan(),
                                style(pinned).dim(),
                                style(tag).green(),
                                style(format!(
                                    "fledge plugin install {}@{} --force",
                                    entry.source, tag
                                ))
                                .cyan()
                            );
                        }
                        results.push(serde_json::json!({
                            "name": entry.name,
                            "status": "skipped",
                            "detail": format!("pinned to {pinned}, latest tag is {tag} — reinstall to upgrade"),
                            "pinned_ref": pinned,
                            "latest_tag": tag,
                        }));
                    }
                }
                _ => {
                    if !json {
                        println!(
                            "  {} {} — pinned to {}, already up to date.",
                            style("✅").green().bold(),
                            style(&entry.name).green(),
                            style(pinned).dim()
                        );
                    }
                    results.push(serde_json::json!({
                        "name": entry.name,
                        "status": "skipped",
                        "detail": format!("pinned to {pinned}, already up to date"),
                        "pinned_ref": pinned,
                    }));
                }
            }
            continue;
        }

        let sp = if json {
            None
        } else {
            Some(crate::spinner::Spinner::start(&format!(
                "Updating {}:",
                &entry.name
            )))
        };

        let mut cmd = Command::new("git");
        cmd.args(["pull", "--ff-only"])
            .current_dir(&plugin_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        apply_git_auth(&mut cmd);

        let status = cmd
            .status()
            .with_context(|| format!("updating {}", entry.name))?;

        if let Some(s) = sp {
            s.finish();
        }

        if !status.success() {
            if !json {
                println!(
                    "  {} {} — git pull failed, try reinstalling with {}",
                    style("⚠️").yellow(),
                    style(&entry.name).yellow(),
                    style(format!("fledge plugin install {} --force", entry.source)).cyan()
                );
            }
            results.push(serde_json::json!({
                "name": entry.name,
                "status": "failed",
                "detail": "git pull failed — reinstall required",
            }));
            continue;
        }

        let result = rebuild_after_fetch(entry, &plugin_dir, None, json)?;
        results.push(result);
    }

    if json {
        let total = results.len();
        let count = |s: &str| results.iter().filter(|r| r["status"] == s).count();
        let summary = serde_json::json!({
            "total": total,
            "updated": count("updated"),
            "skipped": count("skipped"),
            "failed": count("failed"),
        });
        let scope = if defaults {
            "defaults"
        } else if name.is_some() {
            "single"
        } else {
            "all"
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": PLUGINS_UPDATE_SCHEMA,
                "action": "update",
                "scope": scope,
                "results": results,
                "summary": summary,
            }))?
        );
    }

    Ok(())
}

/// Rebuild a plugin after a fetch or checkout. Parses the manifest, checks
/// for new capabilities, rebuilds, relinks commands, and updates the
/// registry. When `new_pinned_ref` is `Some`, the registry's `pinned_ref`
/// is updated to the new tag (used when upgrading pinned defaults).
fn rebuild_after_fetch(
    entry: &super::PluginEntry,
    plugin_dir: &Path,
    new_pinned_ref: Option<&str>,
    json: bool,
) -> Result<serde_json::Value> {
    let manifest_path = plugin_dir.join("plugin.toml");
    if !manifest_path.exists() {
        return Ok(serde_json::json!({
            "name": entry.name,
            "status": "updated",
            "detail": "no plugin.toml — update applied",
        }));
    }

    let manifest_content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let manifest: PluginManifest =
        toml::from_str(&manifest_content).context("parsing plugin.toml")?;

    let new_caps = &manifest.capabilities;
    let old_caps = entry.capabilities.as_ref();
    let added_exec = new_caps.exec && !old_caps.is_some_and(|c| c.exec);
    let added_store = new_caps.store && !old_caps.is_some_and(|c| c.store);
    let added_metadata = new_caps.metadata && !old_caps.is_some_and(|c| c.metadata);
    let has_new_caps = added_exec || added_store || added_metadata;

    if has_new_caps {
        if !json {
            println!(
                "\n  {} {} v{} requests new capabilities:",
                style("!").yellow().bold(),
                style(&entry.name).cyan(),
                manifest.plugin.version
            );
            if added_exec {
                println!("    {} exec — run shell commands", style("+").yellow());
            }
            if added_store {
                println!(
                    "    {} store — persist data between runs",
                    style("+").yellow()
                );
            }
            if added_metadata {
                println!(
                    "    {} metadata — read project metadata and environment",
                    style("+").yellow()
                );
            }
            println!();
        }

        if !crate::utils::is_interactive() {
            return Ok(serde_json::json!({
                "name": entry.name,
                "status": "failed",
                "detail": "update adds new capabilities — rerun interactively or reinstall with --force",
            }));
        }

        let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Grant new capabilities?")
            .default(false)
            .interact()?;
        if !confirm {
            return Ok(serde_json::json!({
                "name": entry.name,
                "status": "skipped",
                "detail": "new capabilities declined by user",
            }));
        }
    }

    run_build(plugin_dir, &manifest)?;

    let bin_dir = plugin_bin_dir();
    for old_cmd in &entry.commands {
        let old_link = bin_dir.join(format!("fledge-{old_cmd}"));
        if old_link.exists() || old_link.is_symlink() {
            fs::remove_file(&old_link).ok();
        }
    }
    link_commands(plugin_dir, &bin_dir, &manifest)?;

    let granted_caps = if manifest.plugin.protocol.is_some() {
        Some(manifest.capabilities.clone())
    } else {
        entry.capabilities.clone()
    };

    let new_cmds: Vec<String> = manifest.commands.iter().map(|c| c.name.clone()).collect();
    let mut reg = load_registry()?;
    if let Some(e) = reg.plugins.iter_mut().find(|p| p.name == entry.name) {
        e.version = manifest.plugin.version.clone();
        e.commands = new_cmds.clone();
        e.capabilities = granted_caps;
        if let Some(ref_tag) = new_pinned_ref {
            e.pinned_ref = Some(ref_tag.to_string());
        }
    }
    save_registry(&reg)?;

    if !json {
        if let Some(ref_tag) = new_pinned_ref {
            let old_ref = entry.pinned_ref.as_deref().unwrap_or("?");
            println!(
                "  {} {} {} → {} (v{})",
                style("✅").green().bold(),
                style(&entry.name).green(),
                style(old_ref).dim(),
                style(ref_tag).green(),
                manifest.plugin.version
            );
        } else {
            println!(
                "  {} {} → v{}",
                style("✅").green().bold(),
                style(&entry.name).green(),
                manifest.plugin.version
            );
        }
    }

    let mut result = serde_json::json!({
        "name": entry.name,
        "status": "updated",
        "version": manifest.plugin.version,
        "commands": new_cmds,
    });
    if let Some(ref_tag) = new_pinned_ref {
        result["previous_ref"] = serde_json::json!(entry.pinned_ref);
        result["new_ref"] = serde_json::json!(ref_tag);
    }

    Ok(result)
}

/// True when `plugin_dir` is the root of a git working tree. We accept both
/// `.git` directories (standard clones) and `.git` files (worktrees /
/// submodules). Plugins installed from a monorepo and symlinked into place
/// will fail this check, which is exactly what we want — workspace plugins
/// should not be `git pull`ed (issue #382).
fn is_git_repo(plugin_dir: &Path) -> bool {
    plugin_dir.join(".git").exists()
}

pub(crate) fn find_latest_tag(repo_dir: &Path) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(["fetch", "--tags"])
        .current_dir(repo_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    apply_git_auth(&mut cmd);

    cmd.status().ok();
    let output = Command::new("git")
        .args(["tag", "--sort=-v:refname"])
        .current_dir(repo_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::is_git_repo;
    use tempfile::TempDir;

    #[test]
    fn is_git_repo_detects_dot_git_directory() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_git_repo(tmp.path()));
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn is_git_repo_detects_dot_git_file_for_worktrees_or_submodules() {
        // `git worktree add` and `git submodule` both create a `.git` file
        // (not a directory). Both forms must register as a real repo.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn is_git_repo_returns_false_for_workspace_managed_plugin() {
        // Issue #382: a plugin symlinked from a monorepo (or copied into the
        // plugins dir without its own git metadata) has no `.git` entry.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("plugin.toml"), "[plugin]\nname = \"x\"\n").unwrap();
        assert!(!is_git_repo(tmp.path()));
    }
}
