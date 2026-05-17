use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::process::Command;

use crate::trust::{determine_trust_tier, parse_source_ref, TrustTier};

use super::{
    apply_git_auth, extract_name_from_source, link_commands, load_registry, normalize_source,
    plugin_bin_dir, plugins_dir, run_build, run_hook, save_registry, validate_plugin_name,
    PluginCapabilities, PluginEntry, PluginManifest, PLUGINS_INSTALL_SCHEMA,
};

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

/// Top-level dispatcher for `fledge plugins install`. Splits the
/// single-source path from the `--defaults` bulk-install path so each
/// caller stays simple. Reports a per-plugin pass/fail count when
/// installing the bundle so a single bad repo doesn't abort the rest.
pub(crate) fn install_action(
    source: Option<&str>,
    force: bool,
    defaults: bool,
    json: bool,
) -> Result<()> {
    if defaults {
        if source.is_some() {
            bail!("--defaults installs the curated set; do not pass a source ref alongside it.");
        }
        return install_defaults(force, json);
    }
    let source = source.ok_or_else(|| {
        anyhow::anyhow!(
            "Either pass a source ref (owner/repo[@ref]) or use --defaults to install the curated set."
        )
    })?;
    let report = install_plugin(source, force, json)?;
    if json {
        let result = serde_json::json!({
            "schema_version": PLUGINS_INSTALL_SCHEMA,
            "action": "install",
            "scope": "single",
            "installed": [report],
            "failed": [],
            "summary": { "total": 1, "installed": 1, "failed": 0 },
        });
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
        match install_plugin(source, force, json) {
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
        let result = serde_json::json!({
            "schema_version": PLUGINS_INSTALL_SCHEMA,
            "action": "install",
            "scope": "defaults",
            "installed": installed,
            "failed": failed_json,
            "summary": {
                "total": DEFAULT_PLUGINS.len(),
                "installed": installed_sources.len(),
                "failed": failed.len(),
            },
        });
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
pub(crate) fn install_plugin(source: &str, force: bool, json: bool) -> Result<serde_json::Value> {
    let force = force || crate::utils::is_non_interactive();
    let (_, git_ref) = parse_source_ref(source);
    let url = normalize_source(source);
    let repo_name = extract_name_from_source(source);
    validate_plugin_name(&repo_name)?;

    let tier = determine_trust_tier(source);
    if !json {
        println!(
            "\n{} Installing plugin from: {} [{}]",
            style("!").yellow().bold(),
            style(&url).cyan(),
            tier.styled_label()
        );
        if tier == TrustTier::Official {
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

    if !force {
        if !crate::utils::is_interactive() {
            bail!(
                "Plugin installation requires confirmation in non-interactive mode.\n  \
                 Use --yes or --force to skip prompts."
            );
        }
        let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!("Install plugin '{repo_name}' from {url}?"))
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
        fs::remove_dir_all(&plugin_dir).context("removing existing plugin")?;
    }

    let sp = if json {
        None
    } else {
        let clone_msg = match git_ref {
            Some(r) => format!("Cloning {}@{}:", &url, r),
            None => format!("Cloning {}:", &url),
        };
        Some(crate::spinner::Spinner::start(&clone_msg))
    };

    let mut clone_args = vec!["clone"];
    if git_ref.is_none() {
        clone_args.push("--depth");
        clone_args.push("1");
    }
    clone_args.push(&url);

    let mut cmd = Command::new("git");
    cmd.args(&clone_args)
        .arg(&plugin_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    apply_git_auth(&mut cmd);

    let status = cmd.status().context("running git clone")?;

    if let Some(s) = sp {
        s.finish();
    }

    if !status.success() {
        bail!(
            "Failed to clone '{}'. Check the repository URL and your network connection.",
            source
        );
    }

    if let Some(ref_str) = git_ref {
        if ref_str.starts_with('-') {
            fs::remove_dir_all(&plugin_dir).ok();
            bail!(
                "Invalid git ref '{}': references cannot start with a hyphen.",
                ref_str
            );
        }
        let status = Command::new("git")
            .args(["checkout", ref_str])
            .current_dir(&plugin_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .with_context(|| format!("checking out ref '{ref_str}'"))?;
        if !status.success() {
            fs::remove_dir_all(&plugin_dir).ok();
            bail!(
                "Git ref '{}' not found in '{}'. Check available tags with:\n  {}",
                ref_str,
                source,
                style(format!("git ls-remote --tags {url}")).cyan()
            );
        }
    }

    let manifest_path = plugin_dir.join("plugin.toml");
    if !manifest_path.exists() {
        fs::remove_dir_all(&plugin_dir).ok();
        bail!(
            "Repository '{}' has no plugin.toml manifest.\n  See {} for the plugin format.",
            source,
            style("https://github.com/CorvidLabs/fledge#plugins").cyan()
        );
    }

    let manifest_content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let manifest: PluginManifest =
        toml::from_str(&manifest_content).context("parsing plugin.toml")?;

    let caps = &manifest.capabilities;
    let has_protocol_caps = caps.exec || caps.store || caps.metadata;
    let has_wasm_caps = caps.filesystem.as_deref().is_some_and(|f| f != "none") || caps.network;
    let has_caps = has_protocol_caps || has_wasm_caps;
    let needs_cap_prompt =
        has_caps && (manifest.plugin.protocol.is_some() || manifest.plugin.is_wasm());
    let has_hooks = manifest.hooks.has_any();

    if let Err(blocked) = check_tier_capabilities(tier, caps) {
        if let Err(e) = fs::remove_dir_all(&plugin_dir) {
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
        if force {
            eprintln!(
                "  {} Permissions auto-granted via --force",
                style("WARN").yellow()
            );
        } else if !crate::utils::is_interactive() {
            fs::remove_dir_all(&plugin_dir).ok();
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
                fs::remove_dir_all(&plugin_dir).ok();
                bail!("Plugin installation cancelled.");
            }
        }
    }

    if let Err(e) = run_build(&plugin_dir, &manifest) {
        fs::remove_dir_all(&plugin_dir).ok();
        return Err(e.context("build failed; installation rolled back"));
    }

    if manifest.plugin.is_wasm() {
        for cmd in &manifest.commands {
            let wasm_path = plugin_dir.join(&cmd.binary);
            if wasm_path.exists() {
                println!(
                    "  {} Pre-compiling WASM module...",
                    style("▶").cyan().bold()
                );
                super::wasm::compile_and_cache(&wasm_path)?;
            } else {
                fs::remove_dir_all(&plugin_dir).ok();
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

    let command_names = link_commands(&plugin_dir, &bin_dir, &manifest).inspect_err(|_| {
        fs::remove_dir_all(&plugin_dir).ok();
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
            fs::remove_dir_all(&plugin_dir).ok();
            return Err(e.context("post_install hook failed; installation rolled back"));
        }
    }

    let (base_source, _) = parse_source_ref(source);
    let granted_caps = if manifest.plugin.protocol.is_some() {
        Some(manifest.capabilities.clone())
    } else {
        None
    };
    let entry = PluginEntry {
        name: repo_name.clone(),
        source: base_source.to_string(),
        version: manifest.plugin.version.clone(),
        installed: chrono::Local::now().format("%Y-%m-%d").to_string(),
        commands: command_names.clone(),
        pinned_ref: git_ref.map(String::from),
        capabilities: granted_caps,
        runtime: manifest.plugin.runtime.clone(),
    };

    if let Some(idx) = existing {
        registry.plugins[idx] = entry.clone();
    } else {
        registry.plugins.push(entry.clone());
    }
    save_registry(&registry)?;

    if !json {
        if let Some(ref pinned) = git_ref {
            println!(
                "{} Installed {} v{} (pinned to {})",
                style("✅").green().bold(),
                style(&manifest.plugin.name).green(),
                manifest.plugin.version,
                style(pinned).cyan()
            );
        } else {
            println!(
                "{} Installed {} v{}",
                style("✅").green().bold(),
                style(&manifest.plugin.name).green(),
                manifest.plugin.version
            );
        }
        if !command_names.is_empty() {
            println!("  Commands: {}", style(command_names.join(", ")).cyan());
        }
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
