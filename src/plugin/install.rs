use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::trust::{determine_trust_tier, parse_source_ref, TrustTier};

use super::{
    apply_git_auth, copy_dir_all, create_dir_symlink, extract_name_from_source, link_commands,
    load_registry, normalize_source, plugin_bin_dir, plugins_dir, remove_plugin_path, run_build,
    run_hook, save_registry, validate_plugin_name, PluginCapabilities, PluginEntry, PluginManifest,
    PLUGINS_INSTALL_SCHEMA,
};

#[derive(Debug)]
pub(super) enum InstallSource {
    LocalPath {
        original: String,
        canonical: PathBuf,
        copy: bool,
    },
    Git {
        original: String,
        clone_url: String,
        registry_source: String,
        git_ref: Option<String>,
        repo_name: String,
    },
}

impl InstallSource {
    pub(super) fn parse(source: &str, copy: bool) -> Result<Self> {
        if let Some(local) = Self::local_path(source, copy)? {
            return Ok(local);
        }
        if copy {
            bail!("--copy is only valid when installing from a local path.");
        }

        // Reject "." / ".." path segments in a remote source. git/curl collapse
        // them client-side (RFC 3986), so "CorvidLabs/../attacker/evil" would be
        // fetched as "attacker/evil" while trust classification keys on the
        // official "CorvidLabs" org — a trust-tier spoof (review finding H-2).
        if crate::trust::source_has_path_traversal(source) {
            bail!("Invalid plugin source '{source}': '.' and '..' path segments are not allowed.");
        }

        let (base, git_ref) = parse_source_ref(source);
        let clone_url = if Self::is_git_url(base) {
            base.to_string()
        } else {
            normalize_source(source)
        };
        let repo_name = extract_name_from_source(source);
        validate_plugin_name(&repo_name)?;
        Ok(Self::Git {
            original: source.to_string(),
            clone_url,
            registry_source: base.to_string(),
            git_ref: git_ref.map(String::from),
            repo_name,
        })
    }

    fn local_path(source: &str, copy: bool) -> Result<Option<Self>> {
        if source.starts_with("file://") {
            return Ok(None);
        }
        let path = Path::new(source);
        let looks_like_path = path.is_absolute()
            || source.starts_with("./")
            || source.starts_with("../")
            || source == "."
            || source == "..";
        if !looks_like_path && !path.exists() {
            return Ok(None);
        }
        if !path.exists() {
            bail!("Local plugin path '{}' does not exist.", source);
        }
        let canonical = path
            .canonicalize()
            .with_context(|| format!("resolving local plugin path '{}'", source))?;
        if !canonical.is_dir() {
            bail!("Local plugin path '{}' is not a directory.", source);
        }
        Ok(Some(Self::LocalPath {
            original: source.to_string(),
            canonical,
            copy,
        }))
    }

    fn is_git_url(source: &str) -> bool {
        source.starts_with("http://")
            || source.starts_with("https://")
            || source.starts_with("ssh://")
            || source.starts_with("git://")
            || source.starts_with("file://")
            || source.starts_with("git@")
    }

    fn install_name(&self) -> Result<String> {
        match self {
            Self::LocalPath { canonical, .. } => {
                let manifest = read_manifest(canonical)?;
                validate_plugin_name(&manifest.plugin.name)?;
                Ok(manifest.plugin.name)
            }
            Self::Git { repo_name, .. } => Ok(repo_name.clone()),
        }
    }

    fn registry_source(&self) -> String {
        match self {
            Self::LocalPath { canonical, .. } => canonical.to_string_lossy().to_string(),
            Self::Git {
                registry_source, ..
            } => registry_source.clone(),
        }
    }

    fn display_source(&self) -> String {
        match self {
            Self::LocalPath {
                original,
                canonical,
                copy,
            } => {
                if *copy {
                    format!("{} ({}, copied)", canonical.display(), original)
                } else {
                    format!("{} ({}, linked)", canonical.display(), original)
                }
            }
            Self::Git { clone_url, .. } => clone_url.clone(),
        }
    }

    fn trust_tier(&self) -> TrustTier {
        match self {
            Self::LocalPath { .. } => TrustTier::Local,
            Self::Git { original, .. } => determine_trust_tier(original),
        }
    }
}

pub(super) fn check_tier_capabilities(
    tier: TrustTier,
    caps: &PluginCapabilities,
) -> std::result::Result<(), Vec<&'static str>> {
    if tier != TrustTier::Unverified {
        return Ok(());
    }
    let mut blocked = Vec::new();
    if caps.exec {
        blocked.push("exec");
    }
    if caps.network {
        blocked.push("network");
    }
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(blocked)
    }
}

fn read_manifest(plugin_dir: &Path) -> Result<PluginManifest> {
    let manifest_path = plugin_dir.join("plugin.toml");
    if !manifest_path.exists() {
        bail!(
            "Plugin directory '{}' has no plugin.toml manifest.\n  See {} for the plugin format.",
            plugin_dir.display(),
            style("https://github.com/CorvidLabs/fledge#plugins").cyan()
        );
    }
    let manifest_content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    toml::from_str(&manifest_content).context("parsing plugin.toml")
}

fn materialize_source(source: &InstallSource, plugin_dir: &Path, json: bool) -> Result<()> {
    match source {
        InstallSource::LocalPath {
            canonical, copy, ..
        } => {
            if *copy {
                if !json {
                    println!(
                        "  {} Copying local plugin from {}",
                        style("→").dim(),
                        style(canonical.display()).cyan()
                    );
                }
                copy_dir_all(canonical, plugin_dir)
            } else {
                if !json {
                    println!(
                        "  {} Linking local plugin from {}",
                        style("→").dim(),
                        style(canonical.display()).cyan()
                    );
                }
                create_dir_symlink(canonical, plugin_dir)
            }
        }
        InstallSource::Git {
            original,
            clone_url,
            git_ref,
            ..
        } => clone_git_source(original, clone_url, git_ref.as_deref(), plugin_dir, json),
    }
}

fn clone_git_source(
    original: &str,
    clone_url: &str,
    git_ref: Option<&str>,
    plugin_dir: &Path,
    json: bool,
) -> Result<()> {
    let sp = if json {
        None
    } else {
        let clone_msg = match git_ref {
            Some(r) => format!("Cloning {}@{}:", clone_url, r),
            None => format!("Cloning {}:", clone_url),
        };
        Some(crate::spinner::Spinner::start(&clone_msg))
    };

    let mut clone_args = vec!["clone"];
    if git_ref.is_none() {
        clone_args.push("--depth");
        clone_args.push("1");
    }
    clone_args.push(clone_url);

    let mut cmd = Command::new("git");
    cmd.args(&clone_args)
        .arg(plugin_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if is_github_clone_url(clone_url) {
        apply_git_auth(&mut cmd);
    }

    let status = cmd.status().context("running git clone")?;

    if let Some(s) = sp {
        s.finish();
    }

    if !status.success() {
        bail!(
            "Failed to clone '{}'. Check the repository URL and your network connection.",
            original
        );
    }

    if let Some(ref_str) = git_ref {
        if ref_str.starts_with('-') {
            remove_plugin_path(plugin_dir).ok();
            bail!(
                "Invalid git ref '{}': references cannot start with a hyphen.",
                ref_str
            );
        }
        let status = Command::new("git")
            .args(["checkout", ref_str])
            .current_dir(plugin_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| format!("checking out ref '{ref_str}'"))?;
        if !status.success() {
            remove_plugin_path(plugin_dir).ok();
            bail!(
                "Git ref '{}' not found in '{}'. Check available tags with:\n  {}",
                ref_str,
                original,
                style(format!("git ls-remote --tags {clone_url}")).cyan()
            );
        }
    }

    Ok(())
}

fn is_github_clone_url(clone_url: &str) -> bool {
    clone_url.starts_with("https://github.com/") || clone_url.starts_with("git@github.com:")
}

/// Top-level dispatcher for `fledge plugins install`. Splits the
/// single-source path from the `--defaults` bulk-install path so each
/// caller stays simple. Reports a per-plugin pass/fail count when
/// installing the bundle so a single bad repo doesn't abort the rest.
pub(crate) fn install_action(
    source: Option<&str>,
    force: bool,
    copy: bool,
    defaults: bool,
    json: bool,
) -> Result<()> {
    if defaults {
        if source.is_some() {
            bail!("--defaults installs the curated set; do not pass a source ref alongside it.");
        }
        if copy {
            bail!("--copy cannot be used with --defaults.");
        }
        return install_defaults(force, json);
    }
    let source = source.ok_or_else(|| {
        anyhow::anyhow!(
            "Either pass a source ref (owner/repo[@ref]) or use --defaults to install the curated set."
        )
    })?;
    let report = install_plugin(source, force, copy, json)?;
    if json {
        let result = crate::envelope::action(
            PLUGINS_INSTALL_SCHEMA,
            "install",
            serde_json::json!({
                "scope": "single",
                "installed": [report],
                "failed": [],
                "summary": { "total": 1, "installed": 1, "failed": 0 },
            }),
        );
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

/// Install every entry in `DEFAULT_PLUGINS`. Failures are collected and
/// reported at the end — one broken default doesn't block the rest, so
/// users on slow networks or with one transient 403 still get the
/// remaining plugins installed.
pub(crate) fn install_defaults(force: bool, json: bool) -> Result<()> {
    use super::DEFAULT_PLUGINS;

    if !json {
        println!(
            "{} Installing {} default plugins...",
            style("*").cyan().bold(),
            DEFAULT_PLUGINS.len()
        );
    }

    let mut installed: Vec<serde_json::Value> = Vec::new();
    let mut installed_sources: Vec<&str> = Vec::new();
    let mut failed: Vec<(&str, String)> = Vec::new();

    for source in DEFAULT_PLUGINS {
        if !json {
            println!();
            println!("  {} {}", style("→").dim(), style(source).cyan());
        }
        match install_plugin(source, force, false, json) {
            Ok(report) => {
                installed.push(report);
                installed_sources.push(source);
            }
            Err(e) => failed.push((source, e.to_string())),
        }
    }

    if !json {
        println!();
        println!(
            "{} {} of {} default plugins installed.",
            if failed.is_empty() {
                style("✅").green().bold()
            } else {
                style("⚠️").yellow().bold()
            },
            installed_sources.len(),
            DEFAULT_PLUGINS.len()
        );

        if !failed.is_empty() {
            println!();
            println!("Failures:");
            for (source, err) in &failed {
                println!("  {} {} — {}", style("✗").red(), style(source).cyan(), err);
            }
        }
    }

    if json {
        let failed_json: Vec<serde_json::Value> = failed
            .iter()
            .map(|(source, err)| serde_json::json!({ "source": source, "error": err }))
            .collect();
        let result = crate::envelope::action(
            PLUGINS_INSTALL_SCHEMA,
            "install",
            serde_json::json!({
                "scope": "defaults",
                "installed": installed,
                "failed": failed_json,
                "summary": {
                    "total": DEFAULT_PLUGINS.len(),
                    "installed": installed_sources.len(),
                    "failed": failed.len(),
                },
            }),
        );
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    if !failed.is_empty() {
        bail!("{} default plugin(s) failed to install.", failed.len());
    }

    Ok(())
}

/// Install a single plugin. Returns a JSON-serializable report describing
/// what was installed; the caller is responsible for printing the JSON
/// envelope (so single-install and bulk-install share one shape).
pub(crate) fn install_plugin(
    source: &str,
    force: bool,
    copy: bool,
    json: bool,
) -> Result<serde_json::Value> {
    let force = force || crate::utils::is_non_interactive();
    let install_source = InstallSource::parse(source, copy)?;
    let repo_name = install_source.install_name()?;
    let display_source = install_source.display_source();

    let tier = install_source.trust_tier();
    if !json {
        print_install_banner(&display_source, tier);
    }

    if !force {
        if !crate::utils::is_interactive() {
            bail!(
                "Plugin installation requires confirmation in non-interactive mode.\n  \
                 Use --yes or --force to skip prompts."
            );
        }
        let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!(
                "Install plugin '{repo_name}' from {display_source}?"
            ))
            .default(true)
            .interact()?;
        if !confirm {
            bail!("Plugin installation cancelled.");
        }
    }

    let plugins = plugins_dir();
    let bin_dir = plugin_bin_dir();
    fs::create_dir_all(&plugins)?;
    fs::create_dir_all(&bin_dir)?;

    let plugin_dir = plugins.join(&repo_name);

    let mut registry = load_registry()?;
    let existing = registry.plugins.iter().position(|p| p.name == repo_name);

    if plugin_dir.exists() {
        if !force {
            bail!(
                "Plugin '{}' is already installed.\n  Use {} to reinstall.",
                repo_name,
                style("--force").cyan()
            );
        }
        remove_plugin_path(&plugin_dir).context("removing existing plugin")?;
    }

    if let Err(e) = materialize_source(&install_source, &plugin_dir, json) {
        remove_plugin_path(&plugin_dir).ok();
        return Err(e);
    }

    let manifest = match read_manifest(&plugin_dir) {
        Ok(manifest) => manifest,
        Err(e) => {
            remove_plugin_path(&plugin_dir).ok();
            return Err(e);
        }
    };

    let flags = CapabilityFlags::from_manifest(&manifest);
    let needs_cap_prompt = flags.needs_cap_prompt;
    let has_hooks = flags.has_hooks;

    if let Err(blocked) = check_tier_capabilities(tier, &manifest.capabilities) {
        if let Err(e) = remove_plugin_path(&plugin_dir) {
            eprintln!(
                "Warning: failed to clean up partial install at {}: {e}",
                plugin_dir.display()
            );
        }
        bail!(
            "Unverified plugin '{}' requests dangerous capabilities: {}\n  \
             Only official and team-tier plugins may use exec or network.\n  \
             To trust this source, run: fledge config add trust.orgs <owner>\n  \
             Or fork it under an account you control.",
            repo_name,
            blocked.join(", ")
        );
    }

    if needs_cap_prompt || has_hooks {
        if !json {
            print_requested_capabilities(&manifest, needs_cap_prompt, has_hooks);
        }
        if force {
            eprintln!(
                "  {} Permissions auto-granted via --force",
                style("WARN").yellow()
            );
        } else if !crate::utils::is_interactive() {
            remove_plugin_path(&plugin_dir).ok();
            bail!(
                "Plugin permissions require confirmation in non-interactive mode.\n  \
                 Use --yes or --force to auto-grant."
            );
        } else {
            let prompt_msg = if needs_cap_prompt && has_hooks {
                "Grant capabilities and approve hooks?"
            } else if needs_cap_prompt {
                "Grant these capabilities?"
            } else {
                "Approve these hooks?"
            };
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(prompt_msg)
                    .default(true)
                    .interact()?;
            if !confirm {
                remove_plugin_path(&plugin_dir).ok();
                bail!("Plugin installation cancelled.");
            }
        }
    }

    if let Err(e) = run_build(&plugin_dir, &manifest) {
        remove_plugin_path(&plugin_dir).ok();
        return Err(e.context("build failed; installation rolled back"));
    }

    if manifest.plugin.is_wasm() {
        #[cfg(feature = "wasm")]
        {
            for cmd in &manifest.commands {
                let wasm_path = plugin_dir.join(&cmd.binary);
                if wasm_path.exists() {
                    println!(
                        "  {} Pre-compiling WASM module...",
                        style("▶").cyan().bold()
                    );
                    super::wasm::compile_and_cache(&wasm_path)?;
                } else {
                    remove_plugin_path(&plugin_dir).ok();
                    bail!(
                        "WASM binary '{}' not found after build.\n  \
                         Check that the build hook produces a .wasm file at the path declared in plugin.toml.\n  \
                         Expected: {}",
                        cmd.binary,
                        wasm_path.display()
                    );
                }
            }
        }
        #[cfg(not(feature = "wasm"))]
        {
            remove_plugin_path(&plugin_dir).ok();
            bail!(
                "Plugin '{}' requires the WASM runtime, which was not compiled in \
                 (rebuild with --features wasm).",
                manifest.plugin.name
            );
        }
    }

    let command_names = link_commands(&plugin_dir, &bin_dir, &manifest).inspect_err(|_| {
        remove_plugin_path(&plugin_dir).ok();
    })?;

    // Run post_install hook BEFORE persisting to registry so a hook failure
    // doesn't leave the plugin marked as installed but non-functional.
    if let Some(hook) = &manifest.hooks.post_install {
        if let Err(e) = run_hook(&plugin_dir, hook, "post_install") {
            // Roll back: remove symlinks and plugin directory
            for cmd_name in &command_names {
                let link_path = bin_dir.join(format!("fledge-{cmd_name}"));
                fs::remove_file(&link_path).ok();
            }
            remove_plugin_path(&plugin_dir).ok();
            return Err(e.context("post_install hook failed; installation rolled back"));
        }
    }

    let entry = build_plugin_entry(&repo_name, &install_source, &manifest, &command_names);

    if let Some(idx) = existing {
        registry.plugins[idx] = entry.clone();
    } else {
        registry.plugins.push(entry.clone());
    }
    save_registry(&registry)?;

    if !json {
        print_install_success(
            &entry,
            &manifest.plugin.name,
            &manifest.plugin.version,
            &command_names,
        );
    }

    Ok(serde_json::json!({
        "name": entry.name,
        "source": entry.source,
        "version": entry.version,
        "trust_tier": tier.label(),
        "commands": entry.commands,
        "pinned_ref": entry.pinned_ref,
        "capabilities": entry.capabilities,
    }))
}

/// Capability/hook flags derived from a plugin manifest. Pure (no I/O), so the
/// install flow's gating and prompting can read them by name and the derivation
/// is unit-testable without touching the filesystem or network.
struct CapabilityFlags {
    /// The manifest requests a capability AND declares a protocol or wasm
    /// runtime, so the user must be prompted before it is granted.
    needs_cap_prompt: bool,
    /// The manifest defines at least one lifecycle hook.
    has_hooks: bool,
}

impl CapabilityFlags {
    fn from_manifest(manifest: &PluginManifest) -> Self {
        let caps = &manifest.capabilities;
        let has_protocol_caps = caps.exec || caps.store || caps.metadata;
        let has_wasm_caps = caps.filesystem.as_deref().is_some_and(|f| f != "none") || caps.network;
        let has_caps = has_protocol_caps || has_wasm_caps;
        let needs_cap_prompt =
            has_caps && (manifest.plugin.protocol.is_some() || manifest.plugin.is_wasm());
        Self {
            needs_cap_prompt,
            has_hooks: manifest.hooks.has_any(),
        }
    }
}

/// Print the pre-install banner: source, trust tier, and a tier-specific note.
/// Caller gates on `!json`.
fn print_install_banner(display_source: &str, tier: TrustTier) {
    println!(
        "\n{} Installing plugin from: {} [{}]",
        style("!").yellow().bold(),
        style(display_source).cyan(),
        tier.styled_label()
    );
    if tier == TrustTier::Local {
        println!(
            "  {} This is a local plugin. Changes in the source directory are live unless --copy is used.",
            style("✓").magenta()
        );
    } else if tier == TrustTier::Official {
        println!(
            "  {} This is an official CorvidLabs plugin.",
            style("✓").green()
        );
    } else {
        println!(
            "  {} Plugins can execute arbitrary code on your system.",
            style("*").yellow()
        );
        println!(
            "  {} Only install plugins from sources you trust.\n",
            style("*").yellow()
        );
    }
}

/// Print the requested-capabilities and lifecycle-hooks detail shown before the
/// grant prompt. Caller gates on `!json` and on `needs_cap_prompt || has_hooks`.
fn print_requested_capabilities(
    manifest: &PluginManifest,
    needs_cap_prompt: bool,
    has_hooks: bool,
) {
    let caps = &manifest.capabilities;
    if needs_cap_prompt {
        println!("\n  {} Requested capabilities:", style("*").cyan().bold());
        if caps.exec {
            println!("    {} exec — run shell commands", style("•").yellow());
        }
        if caps.store {
            println!(
                "    {} store — persist data between runs",
                style("•").yellow()
            );
        }
        if caps.metadata {
            println!(
                "    {} metadata — read project metadata and environment",
                style("•").yellow()
            );
        }
        if let Some(ref fs_cap) = caps.filesystem {
            match fs_cap.as_str() {
                "project" => {
                    println!(
                        "    {} filesystem (project) — read-only access to project directory",
                        style("•").yellow()
                    );
                }
                "plugin" => {
                    println!(
                        "    {} filesystem (plugin) — read-only project access + read-write plugin data",
                        style("•").yellow()
                    );
                }
                "none" => {}
                other => {
                    println!(
                        "    {} filesystem ({}) — access host files",
                        style("•").yellow(),
                        other
                    );
                }
            }
        }
        if caps.network {
            println!(
                "    {} network — make outbound network requests (unrestricted)",
                style("•").yellow()
            );
        }
        if caps.exec && caps.network {
            println!(
                "\n    {} This plugin can both execute commands and access the network.",
                style("⚠").yellow().bold()
            );
            println!(
                "    {} Together these allow data exfiltration — only install if you trust the source.",
                style("⚠").yellow().bold()
            );
        }
    }
    if has_hooks {
        println!("\n  {} Lifecycle hooks:", style("*").cyan().bold());
        for (name, cmd) in manifest.hooks.iter_defined() {
            println!(
                "    {} {} — {}",
                style("•").yellow(),
                name,
                style(cmd).dim()
            );
        }
    }
    println!();
}

/// Assemble the registry entry recorded for a freshly-installed plugin. The
/// install timestamp is captured here.
fn build_plugin_entry(
    repo_name: &str,
    install_source: &InstallSource,
    manifest: &PluginManifest,
    command_names: &[String],
) -> PluginEntry {
    let pinned_ref = match install_source {
        InstallSource::Git { git_ref, .. } => git_ref.clone(),
        InstallSource::LocalPath { .. } => None,
    };
    let granted_caps = if manifest.plugin.protocol.is_some() {
        Some(manifest.capabilities.clone())
    } else {
        None
    };
    PluginEntry {
        name: repo_name.to_string(),
        source: install_source.registry_source(),
        version: manifest.plugin.version.clone(),
        installed: chrono::Local::now().format("%Y-%m-%d").to_string(),
        commands: command_names.to_vec(),
        pinned_ref,
        capabilities: granted_caps,
        runtime: manifest.plugin.runtime.clone(),
    }
}

/// Print the post-install success summary. Caller gates on `!json`.
fn print_install_success(
    entry: &PluginEntry,
    plugin_name: &str,
    version: &str,
    command_names: &[String],
) {
    if let Some(ref pinned) = entry.pinned_ref {
        println!(
            "{} Installed {} v{} (pinned to {})",
            style("✅").green().bold(),
            style(plugin_name).green(),
            version,
            style(pinned).cyan()
        );
    } else {
        println!(
            "{} Installed {} v{}",
            style("✅").green().bold(),
            style(plugin_name).green(),
            version
        );
    }
    if !command_names.is_empty() {
        println!("  Commands: {}", style(command_names.join(", ")).cyan());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_dotdot_source() {
        // Trust-tier spoof sources must be rejected before classification or
        // clone, whether shorthand or full URL (review finding H-2).
        assert!(InstallSource::parse("CorvidLabs/../attacker/evil", false).is_err());
        assert!(
            InstallSource::parse("https://github.com/CorvidLabs/../attacker/evil.git", false)
                .is_err()
        );
    }

    #[test]
    fn parse_accepts_normal_shorthand() {
        // A legitimate owner/repo shorthand still parses (no network in parse).
        assert!(InstallSource::parse("CorvidLabs/fledge-plugin-deploy", false).is_ok());
    }

    fn manifest(toml_str: &str) -> PluginManifest {
        toml::from_str(toml_str).expect("valid plugin.toml")
    }

    #[test]
    fn capability_flags_bare_manifest_needs_no_prompt() {
        let m = manifest("[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n");
        let flags = CapabilityFlags::from_manifest(&m);
        assert!(!flags.needs_cap_prompt);
        assert!(!flags.has_hooks);
    }

    #[test]
    fn capability_flags_protocol_plugin_with_exec_needs_prompt() {
        let m = manifest(
            "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\nprotocol = \"fledge-v1\"\n\
             [capabilities]\nexec = true\n",
        );
        assert!(CapabilityFlags::from_manifest(&m).needs_cap_prompt);
    }

    #[test]
    fn capability_flags_caps_without_protocol_or_wasm_no_prompt() {
        // Capabilities alone must NOT trigger the grant prompt — the plugin must
        // also be a protocol or wasm plugin that can actually use them.
        let m = manifest(
            "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n\
             [capabilities]\nexec = true\nnetwork = true\n",
        );
        assert!(!CapabilityFlags::from_manifest(&m).needs_cap_prompt);
    }

    #[test]
    fn capability_flags_wasm_plugin_with_network_needs_prompt() {
        let m = manifest(
            "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\nruntime = \"wasm\"\n\
             [capabilities]\nnetwork = true\n",
        );
        assert!(CapabilityFlags::from_manifest(&m).needs_cap_prompt);
    }

    #[test]
    fn capability_flags_filesystem_none_is_not_a_capability() {
        // filesystem = "none" is the absence of filesystem access, not a request.
        let m = manifest(
            "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\nruntime = \"wasm\"\n\
             [capabilities]\nfilesystem = \"none\"\n",
        );
        assert!(!CapabilityFlags::from_manifest(&m).needs_cap_prompt);
    }

    #[test]
    fn capability_flags_filesystem_project_is_a_capability() {
        let m = manifest(
            "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\nruntime = \"wasm\"\n\
             [capabilities]\nfilesystem = \"project\"\n",
        );
        assert!(CapabilityFlags::from_manifest(&m).needs_cap_prompt);
    }

    #[test]
    fn capability_flags_detects_hooks_independently() {
        let m =
            manifest("[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n[hooks]\nbuild = \"make\"\n");
        let flags = CapabilityFlags::from_manifest(&m);
        assert!(flags.has_hooks);
        assert!(!flags.needs_cap_prompt);
    }
}
