use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::{
    apply_git_auth, check_tier_capabilities, link_commands, load_registry, normalize_source,
    plugin_bin_dir, plugins_dir, run_build, save_registry, PluginCapabilities, PluginManifest,
    PLUGINS_UPDATE_SCHEMA,
};
use crate::trust::{determine_trust_tier, parse_source_ref, TrustTier};

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
                    serde_json::to_string_pretty(&build_update_envelope("defaults", &[]))?
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
                            serde_json::to_string_pretty(&build_update_envelope("all", &[]))?
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
        results.push(update_one_plugin(entry, defaults, json)?);
    }

    if json {
        let scope = if defaults {
            "defaults"
        } else if name.is_some() {
            "single"
        } else {
            "all"
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&build_update_envelope(scope, &results))?
        );
    }

    Ok(())
}

/// Update a single installed plugin and return its result record (the value
/// pushed into the `plugins update` results array). Handles the skip cases
/// (missing dir, local, workspace-managed), the pinned-ref upgrade path, and
/// the plain `git pull --ff-only` path; delegates the rebuild to
/// [`rebuild_after_fetch`]. Errors propagate to abort the whole update run,
/// matching the pre-extraction `?` behavior.
fn update_one_plugin(
    entry: &super::PluginEntry,
    defaults: bool,
    json: bool,
) -> Result<serde_json::Value> {
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
        return Ok(serde_json::json!({
            "name": entry.name,
            "status": "failed",
            "detail": "directory missing — reinstall required",
        }));
    }

    if determine_trust_tier(&entry.source) == TrustTier::Local {
        if !json {
            println!(
                "  {} {} — local plugin, skipped",
                style("*").cyan().bold(),
                style(&entry.name).cyan()
            );
        }
        return Ok(serde_json::json!({
            "name": entry.name,
            "status": "skipped",
            "detail": "local plugin — reinstall to refresh copied installs; live-linked installs use the source directory directly",
        }));
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
        return Ok(serde_json::json!({
            "name": entry.name,
            "status": "skipped",
            "detail": "workspace-managed plugin (no .git in install dir) — managed by host project's init step",
        }));
    }

    if let Some(ref pinned) = entry.pinned_ref {
        let latest = find_latest_tag(&plugin_dir, &entry.source);
        match latest {
            Some(ref tag) if tag != pinned => {
                if defaults {
                    let sp = if json {
                        None
                    } else {
                        Some(crate::spinner::Spinner::start(&format!(
                            "Upgrading {} {} → {}:",
                            entry.name, pinned, tag
                        )))
                    };

                    if tag.starts_with('-') {
                        bail!(
                            "Invalid git tag '{}': references cannot start with a hyphen.",
                            tag
                        );
                    }
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
                        return Ok(serde_json::json!({
                            "name": entry.name,
                            "status": "failed",
                            "detail": format!("git checkout {tag} failed — reinstall required"),
                        }));
                    }

                    rebuild_after_fetch(entry, &plugin_dir, Some(tag), json)
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
                    Ok(serde_json::json!({
                        "name": entry.name,
                        "status": "skipped",
                        "detail": format!("pinned to {pinned}, latest tag is {tag} — reinstall to upgrade"),
                        "pinned_ref": pinned,
                        "latest_tag": tag,
                    }))
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
                Ok(serde_json::json!({
                    "name": entry.name,
                    "status": "skipped",
                    "detail": format!("pinned to {pinned}, already up to date"),
                    "pinned_ref": pinned,
                }))
            }
        }
    } else {
        let sp = if json {
            None
        } else {
            Some(crate::spinner::Spinner::start(&format!(
                "Updating {}:",
                entry.name
            )))
        };

        let mut cmd = Command::new("git");
        cmd.args(["pull", "--ff-only"])
            .current_dir(&plugin_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        if is_github_source(&entry.source) {
            apply_git_auth(&mut cmd);
        }

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
            return Ok(serde_json::json!({
                "name": entry.name,
                "status": "failed",
                "detail": "git pull failed — reinstall required",
            }));
        }

        rebuild_after_fetch(entry, &plugin_dir, None, json)
    }
}

/// Build the `plugins update` result envelope: attaches per-status summary
/// counts to the collected results under the given scope. Pure — the summary
/// arithmetic is unit-tested without running a single git command.
fn build_update_envelope(scope: &str, results: &[serde_json::Value]) -> serde_json::Value {
    let count = |status: &str| results.iter().filter(|r| r["status"] == status).count();
    crate::envelope::action(
        PLUGINS_UPDATE_SCHEMA,
        "update",
        serde_json::json!({
            "scope": scope,
            "results": results,
            "summary": {
                "total": results.len(),
                "updated": count("updated"),
                "skipped": count("skipped"),
                "failed": count("failed"),
            },
        }),
    )
}

/// The capability escalation introduced by a freshly-fetched manifest,
/// measured against the caps a plugin was previously granted. Drives both the
/// re-prompt and the trust-tier gate in `rebuild_after_fetch`. Extracted as a
/// pure value so the diff/gate logic is unit-testable without a git checkout.
struct CapabilityDelta {
    added_exec: bool,
    added_store: bool,
    added_metadata: bool,
    added_filesystem: bool,
    added_network: bool,
    /// The new filesystem mode when it was newly granted, for prompt wording.
    new_filesystem: Option<String>,
}

impl CapabilityDelta {
    fn compute(old: Option<&PluginCapabilities>, new: &PluginCapabilities) -> Self {
        // Filesystem is an escalation ladder: none < project < plugin. Only a
        // move UP the ladder grants new host access; a downgrade (e.g.
        // plugin → project, which drops plugin-data write while keeping project
        // read) or a same-level change is not an escalation and must not
        // re-prompt. An unrecognized mode is treated as the most-privileged
        // rung (fail-closed) so a novel mode always re-prompts. A missing entry
        // (`None`) is equivalent to "none". (Kyntrin + Gemini review #437)
        fn fs_rank(mode: &str) -> u8 {
            match mode {
                "none" => 0,
                "project" => 1,
                "plugin" => 2,
                _ => 3,
            }
        }
        let old_fs = old.and_then(|c| c.filesystem.as_deref()).unwrap_or("none");
        let new_fs = new.filesystem.as_deref().unwrap_or("none");
        let added_filesystem = fs_rank(new_fs) > fs_rank(old_fs);
        Self {
            added_exec: new.exec && !old.is_some_and(|c| c.exec),
            added_store: new.store && !old.is_some_and(|c| c.store),
            added_metadata: new.metadata && !old.is_some_and(|c| c.metadata),
            added_filesystem,
            added_network: new.network && !old.is_some_and(|c| c.network),
            new_filesystem: added_filesystem.then(|| new_fs.to_string()),
        }
    }

    fn has_new_caps(&self) -> bool {
        self.added_exec
            || self.added_store
            || self.added_metadata
            || self.added_filesystem
            || self.added_network
    }

    /// The newly-added *dangerous* caps (exec/network) as a `PluginCapabilities`,
    /// so the same `check_tier_capabilities` gate the install path uses can be
    /// re-applied to just the escalation — never to caps that were unchanged.
    fn escalated_dangerous(&self) -> PluginCapabilities {
        PluginCapabilities {
            exec: self.added_exec,
            network: self.added_network,
            ..Default::default()
        }
    }
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

    let delta = CapabilityDelta::compute(entry.capabilities.as_ref(), &manifest.capabilities);

    if delta.has_new_caps() {
        // An update must not silently escalate a plugin past its source's trust
        // tier. Re-run the same gate `install` applies (exec/network are
        // forbidden for unverified sources), scoped to the *newly added*
        // dangerous caps so a plugin whose caps are unchanged still updates.
        let tier = determine_trust_tier(&entry.source);
        if let Err(blocked) = check_tier_capabilities(tier, &delta.escalated_dangerous()) {
            if !json {
                println!(
                    "  {} {} — update requests {} but the source is unverified; refusing to grant.\n    {}",
                    style("⚠️").yellow(),
                    style(&entry.name).yellow(),
                    style(blocked.join(", ")).yellow(),
                    style("Only official and team-tier plugins may use exec or network.").dim()
                );
            }
            return Ok(serde_json::json!({
                "name": entry.name,
                "status": "failed",
                "detail": format!(
                    "update adds {} capability which unverified plugins may not use — reinstall from a trusted source or fork it under an account you control",
                    blocked.join(", ")
                ),
            }));
        }

        // Show the requested capabilities before prompting. In --json mode this
        // goes to stderr so stdout stays a clean JSON document, but an
        // interactive operator still sees exactly what they are being asked to
        // grant (the dialoguer confirm below also renders on stderr). Without
        // this, `--json` on a TTY would prompt to grant caps it never showed.
        // (Gemini review #437)
        let mut cap_lines = vec![format!(
            "\n  {} {} v{} requests new capabilities:",
            style("!").yellow().bold(),
            style(&entry.name).cyan(),
            manifest.plugin.version
        )];
        if delta.added_exec {
            cap_lines.push(format!(
                "    {} exec — run shell commands",
                style("+").yellow()
            ));
        }
        if delta.added_store {
            cap_lines.push(format!(
                "    {} store — persist data between runs",
                style("+").yellow()
            ));
        }
        if delta.added_metadata {
            cap_lines.push(format!(
                "    {} metadata — read project metadata and environment",
                style("+").yellow()
            ));
        }
        if delta.added_filesystem {
            match delta.new_filesystem.as_deref() {
                Some("project") => cap_lines.push(format!(
                    "    {} filesystem (project) — read-only access to project directory",
                    style("+").yellow()
                )),
                Some("plugin") => cap_lines.push(format!(
                    "    {} filesystem (plugin) — read-only project access + read-write plugin data",
                    style("+").yellow()
                )),
                Some(other) => cap_lines.push(format!(
                    "    {} filesystem ({}) — access host files",
                    style("+").yellow(),
                    other
                )),
                None => {}
            }
        }
        if delta.added_network {
            cap_lines.push(format!(
                "    {} network — make outbound network requests (unrestricted)",
                style("+").yellow()
            ));
        }
        let cap_summary = cap_lines.join("\n");
        if json {
            eprintln!("{cap_summary}\n");
        } else {
            println!("{cap_summary}\n");
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

pub(crate) fn find_latest_tag(repo_dir: &Path, source: &str) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(["fetch", "--tags"])
        .current_dir(repo_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if is_github_source(source) {
        apply_git_auth(&mut cmd);
    }

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

fn is_github_source(source: &str) -> bool {
    source.starts_with("https://github.com/")
        || source.starts_with("git@github.com:")
        || (!source.contains("://") && !source.starts_with("git@") && source.contains('/'))
}

#[cfg(test)]
mod tests {
    use super::{
        build_update_envelope, check_tier_capabilities, is_git_repo, CapabilityDelta,
        PluginCapabilities,
    };
    use crate::trust::TrustTier;
    use tempfile::TempDir;

    #[test]
    fn update_envelope_counts_each_status() {
        let results = vec![
            serde_json::json!({ "name": "a", "status": "updated" }),
            serde_json::json!({ "name": "b", "status": "updated" }),
            serde_json::json!({ "name": "c", "status": "skipped" }),
            serde_json::json!({ "name": "d", "status": "failed" }),
        ];
        let env = build_update_envelope("all", &results);
        assert_eq!(env["action"], "update");
        assert_eq!(env["scope"], "all");
        assert_eq!(env["summary"]["total"], 4);
        assert_eq!(env["summary"]["updated"], 2);
        assert_eq!(env["summary"]["skipped"], 1);
        assert_eq!(env["summary"]["failed"], 1);
        assert_eq!(env["results"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn update_envelope_empty_is_all_zero() {
        let env = build_update_envelope("defaults", &[]);
        assert_eq!(env["scope"], "defaults");
        assert_eq!(env["summary"]["total"], 0);
        assert_eq!(env["summary"]["updated"], 0);
        assert_eq!(env["summary"]["skipped"], 0);
        assert_eq!(env["summary"]["failed"], 0);
        assert!(env["results"].as_array().unwrap().is_empty());
    }

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

    fn caps(
        exec: bool,
        store: bool,
        metadata: bool,
        fs: Option<&str>,
        network: bool,
    ) -> PluginCapabilities {
        PluginCapabilities {
            exec,
            store,
            metadata,
            filesystem: fs.map(str::to_string),
            network,
        }
    }

    #[test]
    fn delta_flags_newly_added_network() {
        // The H-3 case: a plugin installed with no network gains it on update.
        let old = caps(false, false, false, None, false);
        let new = caps(false, false, false, None, true);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(d.added_network);
        assert!(d.has_new_caps());
    }

    #[test]
    fn delta_flags_newly_added_filesystem() {
        let old = caps(false, false, false, None, false);
        let new = caps(false, false, false, Some("project"), false);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(d.added_filesystem);
        assert_eq!(d.new_filesystem.as_deref(), Some("project"));
        assert!(d.has_new_caps());
    }

    #[test]
    fn delta_flags_filesystem_escalation_project_to_plugin() {
        // project (read-only) -> plugin (read-write plugin data) is an escalation.
        let old = caps(false, false, false, Some("project"), false);
        let new = caps(false, false, false, Some("plugin"), false);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(d.added_filesystem);
        assert_eq!(d.new_filesystem.as_deref(), Some("plugin"));
    }

    #[test]
    fn delta_ignores_filesystem_downgrade() {
        let old = caps(false, false, false, Some("plugin"), false);
        let new = caps(false, false, false, Some("none"), false);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(!d.added_filesystem);
        assert!(!d.has_new_caps());
    }

    #[test]
    fn delta_ignores_filesystem_downgrade_plugin_to_project() {
        // Kyntrin review #437: plugin -> project drops plugin-data write while
        // keeping project read — a privilege reduction, not an escalation. Must
        // not re-prompt. Escalation is ranked (none < project < plugin), not a
        // bare "changed to non-none" test.
        let old = caps(false, false, false, Some("plugin"), false);
        let new = caps(false, false, false, Some("project"), false);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(!d.added_filesystem);
        assert!(!d.has_new_caps());
    }

    #[test]
    fn delta_unchanged_caps_is_noop() {
        // Preserve behavior for plugins that don't change caps: no prompt, no gate.
        let same = caps(true, true, true, Some("plugin"), true);
        let d = CapabilityDelta::compute(Some(&same), &same);
        assert!(!d.has_new_caps());
        let esc = d.escalated_dangerous();
        assert!(!esc.exec && !esc.network);
    }

    #[test]
    fn unverified_source_blocks_newly_added_network_on_update() {
        let old = caps(false, false, false, None, false);
        let new = caps(false, false, false, None, true);
        let d = CapabilityDelta::compute(Some(&old), &new);
        let res = check_tier_capabilities(TrustTier::Unverified, &d.escalated_dangerous());
        assert_eq!(res.unwrap_err(), vec!["network"]);
    }

    #[test]
    fn unverified_source_blocks_newly_added_exec_on_update() {
        let old = caps(false, false, false, None, false);
        let new = caps(true, false, false, None, false);
        let d = CapabilityDelta::compute(Some(&old), &new);
        let res = check_tier_capabilities(TrustTier::Unverified, &d.escalated_dangerous());
        assert_eq!(res.unwrap_err(), vec!["exec"]);
    }

    #[test]
    fn unverified_source_allows_newly_added_filesystem_on_update() {
        // filesystem is not a tier-gated cap; it is prompt-only even for unverified.
        let old = caps(false, false, false, None, false);
        let new = caps(false, false, false, Some("plugin"), false);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(check_tier_capabilities(TrustTier::Unverified, &d.escalated_dangerous()).is_ok());
    }

    #[test]
    fn official_source_allows_newly_added_exec_and_network_on_update() {
        let old = caps(false, false, false, None, false);
        let new = caps(true, false, false, None, true);
        let d = CapabilityDelta::compute(Some(&old), &new);
        assert!(check_tier_capabilities(TrustTier::Official, &d.escalated_dangerous()).is_ok());
    }

    #[test]
    fn unchanged_dangerous_caps_pass_gate_even_if_source_now_unverified() {
        // Key no-op guarantee: if exec/network were already granted and are
        // unchanged, the delta is empty so the gate does not retroactively block
        // a routine update, even if the source's tier is now unverified.
        let same = caps(true, false, false, None, true);
        let d = CapabilityDelta::compute(Some(&same), &same);
        assert!(check_tier_capabilities(TrustTier::Unverified, &d.escalated_dangerous()).is_ok());
    }

    #[test]
    fn wasm_style_none_old_caps_treats_all_new_caps_as_added() {
        // Registry stores `None` caps for non-protocol (wasm) plugins, so the
        // baseline is empty and any declared cap re-prompts (fail-closed).
        let new = caps(false, false, false, Some("project"), true);
        let d = CapabilityDelta::compute(None, &new);
        assert!(d.added_filesystem && d.added_network);
        assert!(d.has_new_caps());
    }
}
