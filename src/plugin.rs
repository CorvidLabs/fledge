use anyhow::{bail, Context, Result};
use console::style;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::trust::{determine_trust_tier, parse_source_ref, TrustTier};

/// The curated set of plugins fledge endorses as "the default install."
///
/// These are the plugins that took over commands removed from core in
/// v0.15 (the tight-core refactor). Running `fledge plugins install --defaults`
/// installs all of them so a fresh fledge install gets back to feature
/// parity with v0.14 in one command. Each entry is a `<owner>/<repo>` ref
/// understood by `parse_source_ref`.
pub const DEFAULT_PLUGINS: &[&str] = &[
    "CorvidLabs/fledge-plugin-github",
    "CorvidLabs/fledge-plugin-deps",
    "CorvidLabs/fledge-plugin-metrics",
];

#[derive(Debug, Deserialize)]
struct PluginManifest {
    plugin: PluginMeta,
    #[serde(default)]
    commands: Vec<PluginCommand>,
    #[serde(default)]
    hooks: PluginHooks,
    #[serde(default)]
    capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub exec: bool,
    #[serde(default)]
    pub store: bool,
    #[serde(default)]
    pub metadata: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PluginMeta {
    name: String,
    version: String,
    description: Option<String>,
    author: Option<String>,
    protocol: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PluginCommand {
    name: String,
    description: Option<String>,
    binary: String,
}

#[derive(Debug, Deserialize, Default)]
struct PluginHooks {
    build: Option<String>,
    post_install: Option<String>,
    post_remove: Option<String>,
    pre_init: Option<String>,
    post_work_start: Option<String>,
    pre_pr: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PluginsRegistry {
    #[serde(default)]
    plugins: Vec<PluginEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginEntry {
    name: String,
    source: String,
    version: String,
    installed: String,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pinned_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capabilities: Option<PluginCapabilities>,
}

pub struct PluginOptions {
    pub action: PluginAction,
    pub json: bool,
}

pub enum PluginAction {
    Install {
        source: Option<String>,
        force: bool,
        /// Install the curated set of default plugins (DEFAULT_PLUGINS)
        defaults: bool,
    },
    Remove {
        name: String,
    },
    Update {
        name: Option<String>,
        /// Update only the curated set of default plugins (DEFAULT_PLUGINS) — skip community plugins
        defaults: bool,
    },
    List,
    Audit,
    Search {
        query: Option<String>,
        author: Option<String>,
        limit: usize,
    },
    Run {
        name: String,
        args: Vec<String>,
    },
    Publish {
        path: PathBuf,
        org: Option<String>,
        private: bool,
        description: Option<String>,
        yes: bool,
        json: bool,
    },
    Create {
        name: String,
        output: PathBuf,
        description: Option<String>,
        yes: bool,
        json: bool,
    },
    Validate {
        path: PathBuf,
        strict: bool,
        json: bool,
    },
}

pub fn run(opts: PluginOptions) -> Result<()> {
    match opts.action {
        PluginAction::Install {
            source,
            force,
            defaults,
        } => install_action(source.as_deref(), force, defaults, opts.json),
        PluginAction::Remove { name } => remove_plugin(&name, opts.json),
        PluginAction::Update { name, defaults } => {
            update_plugins(name.as_deref(), defaults, opts.json)
        }
        PluginAction::List => list_plugins(opts.json),
        PluginAction::Audit => audit_plugins(opts.json),
        PluginAction::Search {
            query,
            author,
            limit,
        } => search_plugins(query.as_deref(), author.as_deref(), limit, opts.json),
        PluginAction::Run { name, args } => run_plugin(&name, &args),
        PluginAction::Publish {
            path,
            org,
            private,
            description,
            yes,
            json,
        } => publish_plugin(
            &path,
            org.as_deref(),
            private,
            description.as_deref(),
            yes,
            opts.json || json,
        ),
        PluginAction::Create {
            name,
            output,
            description,
            yes,
            json,
        } => create_plugin(
            &name,
            &output,
            description.as_deref(),
            yes,
            opts.json || json,
        ),
        PluginAction::Validate { path, strict, json } => validate_plugin(&path, strict, json),
    }
}

pub fn resolve_plugin_command(name: &str) -> Option<PathBuf> {
    let bin_dir = plugin_bin_dir();
    let bin_path = bin_dir.join(format!("fledge-{name}"));
    if bin_path.exists() {
        return Some(bin_path);
    }

    which_fledge_plugin(name)
}

pub fn run_lifecycle_hook(event: &str) -> Result<()> {
    let registry = load_registry()?;
    for entry in &registry.plugins {
        let plugin_dir = plugins_dir().join(&entry.name);
        let manifest_path = plugin_dir.join("plugin.toml");
        if !manifest_path.exists() {
            continue;
        }
        let content = match fs::read_to_string(&manifest_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let manifest: PluginManifest = match toml::from_str(&content) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let hook = match event {
            "pre_init" => &manifest.hooks.pre_init,
            "post_work_start" => &manifest.hooks.post_work_start,
            "pre_pr" => &manifest.hooks.pre_pr,
            _ => &None,
        };
        if let Some(hook_cmd) = hook {
            println!(
                "  {} {} ({})",
                style("▶️").cyan().bold(),
                style(format!("Plugin hook: {event}")).dim(),
                style(&entry.name).cyan()
            );
            run_hook(&plugin_dir, hook_cmd, &format!("{}/{event}", entry.name))?;
        }
    }
    Ok(())
}

fn apply_git_auth(cmd: &mut Command) {
    let config = crate::config::Config::load().ok();
    let token = config.as_ref().and_then(|c| c.github_token());
    if let Some(ref t) = token {
        use base64::Engine;
        let credentials = format!("x-access-token:{}", t);
        let encoded = base64::engine::general_purpose::STANDARD.encode(&credentials);
        let header_value = format!("Authorization: Basic {}", encoded);
        let existing: usize = std::env::var("GIT_CONFIG_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        cmd.env("GIT_CONFIG_COUNT", (existing + 1).to_string())
            .env(format!("GIT_CONFIG_KEY_{existing}"), "http.extraheader")
            .env(format!("GIT_CONFIG_VALUE_{existing}"), &header_value);
    }
}

fn plugins_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("fledge")
        .join("plugins")
}

fn plugin_bin_dir() -> PathBuf {
    plugins_dir().join("bin")
}

fn registry_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("fledge")
        .join("plugins.toml")
}

fn load_registry() -> Result<PluginsRegistry> {
    let path = registry_path();
    if !path.exists() {
        return Ok(PluginsRegistry {
            plugins: Vec::new(),
        });
    }
    let content = fs::read_to_string(&path).context("reading plugins.toml")?;
    toml::from_str(&content).context("parsing plugins.toml")
}

fn save_registry(registry: &PluginsRegistry) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(registry).context("serializing plugins.toml")?;
    fs::write(&path, content).context("writing plugins.toml")
}

fn normalize_source(source: &str) -> String {
    let (base, _) = parse_source_ref(source);
    if base.starts_with("https://") || base.starts_with("git@") {
        base.to_string()
    } else if base.contains('/') {
        format!("https://github.com/{}.git", base)
    } else {
        base.to_string()
    }
}

fn extract_name_from_source(source: &str) -> String {
    let (base, _) = parse_source_ref(source);
    base.rsplit('/')
        .next()
        .unwrap_or(base)
        .trim_end_matches(".git")
        .to_string()
}

fn detect_build_command(plugin_dir: &Path) -> Option<(&'static str, Vec<&'static str>)> {
    if plugin_dir.join("Cargo.toml").exists() {
        Some(("Rust", vec!["cargo", "build", "--release"]))
    } else if plugin_dir.join("Package.swift").exists() {
        Some(("Swift", vec!["swift", "build", "-c", "release"]))
    } else if plugin_dir.join("go.mod").exists() {
        Some(("Go", vec!["go", "build", "."]))
    } else if plugin_dir.join("package.json").exists() {
        Some(("Node", vec!["npm", "install"]))
    } else {
        None
    }
}

fn run_build(plugin_dir: &Path, manifest: &PluginManifest) -> Result<()> {
    if let Some(hook) = &manifest.hooks.build {
        run_hook(plugin_dir, hook, "build")?;
        return Ok(());
    }

    if let Some((lang, cmd)) = detect_build_command(plugin_dir) {
        let sp = crate::spinner::Spinner::start(&format!("Building ({lang}):"));
        let status = Command::new(cmd[0])
            .args(&cmd[1..])
            .current_dir(plugin_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .with_context(|| format!("running {lang} build"))?;
        sp.finish();
        if !status.success() {
            bail!("Build failed. Check your {lang} toolchain is installed.");
        }
    }

    Ok(())
}

fn validate_command_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.starts_with('.')
        || name.starts_with('-')
        || name == ".."
    {
        bail!(
            "Invalid plugin command name '{}'. Names must be alphanumeric with hyphens/underscores.",
            name
        );
    }
    Ok(())
}

fn link_commands(
    plugin_dir: &Path,
    bin_dir: &Path,
    manifest: &PluginManifest,
) -> Result<Vec<String>> {
    let mut command_names = Vec::new();
    for cmd in &manifest.commands {
        validate_command_name(&cmd.name)?;

        for component in std::path::Path::new(&cmd.binary).components() {
            if matches!(component, std::path::Component::ParentDir) {
                bail!(
                    "Plugin '{}' binary '{}' contains path traversal (..)",
                    manifest.plugin.name,
                    cmd.binary
                );
            }
        }

        let binary_path = plugin_dir.join(&cmd.binary);
        if let Ok(canonical_binary) = binary_path.canonicalize() {
            let canonical_dir = plugin_dir
                .canonicalize()
                .unwrap_or_else(|_| plugin_dir.to_path_buf());
            if !canonical_binary.starts_with(&canonical_dir) {
                bail!(
                    "Plugin '{}' binary '{}' resolves outside the plugin directory",
                    manifest.plugin.name,
                    cmd.binary
                );
            }
        }
        if !binary_path.exists() {
            let mut hint = format!(
                "Plugin '{}' references binary '{}' which does not exist.",
                manifest.plugin.name, cmd.binary
            );
            if let Some((lang, _)) = detect_build_command(plugin_dir) {
                hint.push_str(&format!(
                    "\n  This looks like a {} project. Add a build hook to plugin.toml:",
                    lang
                ));
                hint.push_str("\n  [hooks]");
                let example = match lang {
                    "Rust" => "build = \"cargo build --release\"",
                    "Swift" => "build = \"swift build -c release\"",
                    "Go" => "build = \"go build .\"",
                    _ => "build = \"scripts/build.sh\"",
                };
                hint.push_str(&format!("\n  {example}"));
            }
            bail!("{hint}");
        }

        make_executable(&binary_path)?;

        let link_name = format!("fledge-{}", cmd.name);
        let link_path = bin_dir.join(&link_name);
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path).ok();
        }
        create_symlink(&binary_path, &link_path).with_context(|| {
            format!(
                "creating symlink {} -> {}",
                link_path.display(),
                binary_path.display()
            )
        })?;

        command_names.push(cmd.name.clone());
    }
    Ok(command_names)
}

fn validate_plugin_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('.')
        || name.contains('/')
        || name.contains('\\')
        || name == ".."
    {
        bail!("Invalid plugin source: repo name '{}' is not safe.", name);
    }
    Ok(())
}

/// Top-level dispatcher for `fledge plugins install`. Splits the
/// single-source path from the `--defaults` bulk-install path so each
/// caller stays simple. Reports a per-plugin pass/fail count when
/// installing the bundle so a single bad repo doesn't abort the rest.
fn install_action(source: Option<&str>, force: bool, defaults: bool, json: bool) -> Result<()> {
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
            "schema_version": 1,
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
fn install_defaults(force: bool, json: bool) -> Result<()> {
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
            "schema_version": 1,
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
fn install_plugin(source: &str, force: bool, json: bool) -> Result<serde_json::Value> {
    let force = force || crate::utils::is_non_interactive() || json;
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
    let has_caps = caps.exec || caps.store || caps.metadata;
    if has_caps && manifest.plugin.protocol.is_some() {
        if !json {
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
            println!();
        }
        if force {
            eprintln!(
                "  {} Capabilities auto-granted via --force",
                style("WARN").yellow()
            );
        } else if !crate::utils::is_interactive() {
            fs::remove_dir_all(&plugin_dir).ok();
            bail!(
                "Plugin capabilities require confirmation in non-interactive mode.\n  \
                 Use --yes or --force to auto-grant capabilities."
            );
        } else {
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Grant these capabilities?")
                    .default(true)
                    .interact()?;
            if !confirm {
                fs::remove_dir_all(&plugin_dir).ok();
                bail!("Plugin installation cancelled — capabilities not granted.");
            }
        }
    }

    run_build(&plugin_dir, &manifest)?;

    let command_names = link_commands(&plugin_dir, &bin_dir, &manifest).inspect_err(|_| {
        fs::remove_dir_all(&plugin_dir).ok();
    })?;

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

    if let Some(hook) = &manifest.hooks.post_install {
        run_hook(&plugin_dir, hook, "post_install")?;
    }

    Ok(serde_json::json!({
        "name": entry.name,
        "source": entry.source,
        "version": entry.version,
        "tier": tier.label(),
        "commands": entry.commands,
        "pinned_ref": entry.pinned_ref,
        "capabilities": entry.capabilities,
    }))
}

fn update_plugins(name: Option<&str>, defaults: bool, json: bool) -> Result<()> {
    if defaults && name.is_some() {
        bail!("--defaults updates the curated set; do not pass a plugin name alongside it.");
    }
    let registry = load_registry()?;

    let targets: Vec<&PluginEntry> = if defaults {
        // Match each installed plugin's source against the DEFAULT_PLUGINS
        // list. Stored sources use either the shorthand `owner/repo` form
        // (the install-time input) or the normalized URL — accept both.
        // Plugins from the curated set get updated; every community plugin
        // (figma, weather, e2e, ...) is left alone.
        let is_default = |source: &str| -> bool {
            DEFAULT_PLUGINS.iter().any(|d| {
                source == *d
                    || source == normalize_source(d)
                    || source.trim_end_matches(".git").ends_with(d)
            })
        };
        let matched: Vec<&PluginEntry> = registry
            .plugins
            .iter()
            .filter(|p| is_default(&p.source))
            .collect();
        if matched.is_empty() {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": 1,
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
                                "schema_version": 1,
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

        if let Some(ref pinned) = entry.pinned_ref {
            let latest = find_latest_tag(&plugin_dir);
            match latest {
                Some(ref tag) if tag != pinned => {
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

        let manifest_path = plugin_dir.join("plugin.toml");
        if manifest_path.exists() {
            let manifest_content =
                fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
            let manifest: PluginManifest =
                toml::from_str(&manifest_content).context("parsing plugin.toml")?;

            run_build(&plugin_dir, &manifest)?;

            let bin_dir = plugin_bin_dir();
            for old_cmd in &entry.commands {
                let old_link = bin_dir.join(format!("fledge-{old_cmd}"));
                if old_link.exists() || old_link.is_symlink() {
                    fs::remove_file(&old_link).ok();
                }
            }
            link_commands(&plugin_dir, &bin_dir, &manifest)?;

            let new_cmds: Vec<String> = manifest.commands.iter().map(|c| c.name.clone()).collect();
            let mut reg = load_registry()?;
            if let Some(e) = reg.plugins.iter_mut().find(|p| p.name == entry.name) {
                e.version = manifest.plugin.version.clone();
                e.commands = new_cmds.clone();
            }
            save_registry(&reg)?;

            if !json {
                println!(
                    "  {} {} → v{}",
                    style("✅").green().bold(),
                    style(&entry.name).green(),
                    manifest.plugin.version
                );
            }
            results.push(serde_json::json!({
                "name": entry.name,
                "status": "updated",
                "version": manifest.plugin.version,
                "commands": new_cmds,
            }));
        } else {
            results.push(serde_json::json!({
                "name": entry.name,
                "status": "updated",
                "detail": "no plugin.toml — git pull only",
            }));
        }
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
                "schema_version": 1,
                "action": "update",
                "scope": scope,
                "results": results,
                "summary": summary,
            }))?
        );
    }

    Ok(())
}

fn find_latest_tag(repo_dir: &Path) -> Option<String> {
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

fn remove_plugin(name: &str, json: bool) -> Result<()> {
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
        fs::remove_dir_all(&plugin_dir).context("removing plugin directory")?;
    }

    let removed_name = entry.name.clone();
    let removed_source = entry.source.clone();
    let removed_version = entry.version.clone();
    let removed_commands = entry.commands.clone();
    registry.plugins.remove(idx);
    save_registry(&registry)?;

    if json {
        let result = serde_json::json!({
            "schema_version": 1,
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

fn list_plugins(json: bool) -> Result<()> {
    let registry = load_registry()?;

    if registry.plugins.is_empty() {
        if json {
            println!("[]");
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
        println!("{}", serde_json::to_string_pretty(&entries)?);
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

fn audit_plugins(json: bool) -> Result<()> {
    let registry = load_registry()?;

    if registry.plugins.is_empty() {
        if json {
            println!("[]");
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
        println!("{}", serde_json::to_string_pretty(&entries)?);
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

fn has_lifecycle_hooks(plugin_name: &str) -> bool {
    !get_lifecycle_hooks(plugin_name).is_empty()
}

fn get_lifecycle_hooks(plugin_name: &str) -> Vec<(String, String)> {
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

fn search_plugins(
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
            println!("[]");
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
        println!("{}", serde_json::to_string_pretty(&entries)?);
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

fn run_plugin(name: &str, args: &[String]) -> Result<()> {
    let bin_path = resolve_plugin_command(name)
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
fn resolve_plugin_source_dir(bin_path: &Path) -> Option<PathBuf> {
    let resolved = std::fs::canonicalize(bin_path).ok()?;
    // <plugin_dir>/<bin_subdir>/<binary> — take parent twice.
    resolved.parent()?.parent().map(|p| p.to_path_buf())
}

fn run_hook(plugin_dir: &Path, hook: &str, event: &str) -> Result<()> {
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
        make_executable(&hook_path)?;
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
    resolve_plugin_command(first_cmd)
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
    let content = fs::read_to_string(&manifest_path).ok()?;
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

fn which_fledge_plugin(name: &str) -> Option<PathBuf> {
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

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)?;
        let mut perms = metadata.permissions();
        let mode = perms.mode();
        if mode & 0o111 == 0 {
            perms.set_mode(mode | 0o755);
            fs::set_permissions(path, perms)?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn create_symlink(original: &Path, link: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(original, link)?;
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(original, link)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn create_plugin(
    name: &str,
    output: &Path,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let yes = yes || json || crate::utils::is_non_interactive();
    let target = output.join(name);

    if target.exists() {
        bail!("Directory '{}' already exists", target.display());
    }

    let desc = if yes || !crate::utils::is_interactive() {
        description.unwrap_or("A fledge plugin").to_string()
    } else {
        let theme = dialoguer::theme::ColorfulTheme::default();
        dialoguer::Input::with_theme(&theme)
            .with_prompt("Description")
            .default(description.unwrap_or("A fledge plugin").to_string())
            .interact_text()?
    };

    std::fs::create_dir_all(target.join("bin"))
        .with_context(|| format!("creating {}/bin", target.display()))?;

    let plugin_toml = format!(
        r#"[plugin]
name = {name:?}
version = "0.1.0"
description = {desc:?}
# author = "your-name"

[[commands]]
name = {name:?}
description = {desc:?}
binary = "bin/{name}"

[hooks]
# build = "cargo build --release"
# post_install = "hooks/post-install.sh"

[capabilities]
exec = false
store = false
metadata = false
"#,
    );
    fs::write(target.join("plugin.toml"), plugin_toml).context("writing plugin.toml")?;

    let script = format!(
        r#"#!/usr/bin/env bash
# fledge plugin entry point.
#
# fledge sets FLEDGE_PLUGIN_DIR to this plugin's source directory before
# invoking your binary. Use it to reach sibling files in `bin/`, hooks,
# fixtures, etc. Don't use `dirname "$0"` — the binary fledge invokes is
# a symlink in a shared bin/, so $0 won't point to your repo.
set -euo pipefail
PLUGIN_DIR="${{FLEDGE_PLUGIN_DIR:?FLEDGE_PLUGIN_DIR not set — fledge >= 0.15.3 sets it automatically}}"

echo "{name} plugin running with args: $@"
echo "(plugin dir: $PLUGIN_DIR)"

# To dispatch to sibling helpers in the same `bin/` (a common multi-subcommand
# pattern), use:
#
#   exec "$PLUGIN_DIR/bin/{name}-${{1?missing subcommand}}" "${{@:2}}"
"#
    );
    let script_path = target.join("bin").join(name);
    fs::write(&script_path, script).context("writing bin script")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
            .context("setting executable permission")?;
    }

    fs::write(
        target.join("README.md"),
        format!(
            r#"# {name} — fledge plugin

{desc}

## Install

```bash
fledge plugins install ./{name}
```

Or after publishing:

```bash
fledge plugins install owner/{name}
```

## Commands

| Command | Description |
|---------|-------------|
| `fledge {name}` | {desc} |

## Development

Edit `plugin.toml` to configure commands, hooks, and capabilities.
See [fledge plugin docs](https://github.com/CorvidLabs/fledge) for the full plugin format.
"#
        ),
    )
    .context("writing README.md")?;

    fs::write(
        target.join(".gitignore"),
        "# Build artifacts\n/target/\n/dist/\n\n# OS\n.DS_Store\nThumbs.db\n",
    )
    .context("writing .gitignore")?;

    if json {
        let result = serde_json::json!({
            "schema_version": 1,
            "action": "create",
            "path": target.display().to_string(),
            "name": name,
            "description": desc,
            "files_created": ["plugin.toml", format!("bin/{name}"), "README.md", ".gitignore"],
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "\n{} Created plugin at {}",
            style("✅").green().bold(),
            style(target.display()).cyan()
        );
        println!(
            "\n  {} Edit manifest in {}",
            style("1.").dim(),
            style("plugin.toml").green()
        );
        println!(
            "  {} Validate with: {}",
            style("2.").dim(),
            style(format!("fledge plugins validate ./{name}")).cyan()
        );
        println!(
            "  {} Publish with: {}",
            style("3.").dim(),
            style(format!("fledge plugins publish ./{name}")).cyan()
        );
    }

    Ok(())
}

#[derive(Default, serde::Serialize)]
struct PluginValidationReport {
    path: String,
    plugin_name: String,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_plugin(path: &Path, strict: bool, json: bool) -> Result<()> {
    let path = path.canonicalize().unwrap_or(path.to_path_buf());

    let manifest_path = path.join("plugin.toml");
    if !manifest_path.exists() {
        bail!(
            "No plugin.toml found in {}. Point to a directory containing plugin.toml.",
            path.display()
        );
    }

    let content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let mut report = PluginValidationReport {
        path: path.display().to_string(),
        ..Default::default()
    };

    let manifest: PluginManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            report.errors.push(format!("Invalid plugin.toml: {e}"));
            return print_plugin_report(&report, strict, json);
        }
    };

    report.plugin_name = manifest.plugin.name.clone();

    if manifest.plugin.name.is_empty() {
        report.errors.push("plugin.name is empty".to_string());
    }

    if manifest.plugin.version.is_empty() {
        report.errors.push("plugin.version is empty".to_string());
    } else if crate::versioning::parse_version(&manifest.plugin.version).is_err() {
        report.errors.push(format!(
            "plugin.version is not valid semver: '{}' (expected major.minor.patch)",
            manifest.plugin.version
        ));
    }

    if manifest.plugin.description.is_none() {
        report
            .warnings
            .push("plugin.description is not set".to_string());
    }

    if manifest.plugin.author.is_none() {
        report.warnings.push("plugin.author is not set".to_string());
    }

    if manifest.commands.is_empty() {
        report
            .warnings
            .push("No [[commands]] defined — plugin won't register any subcommands".to_string());
    }

    for cmd in &manifest.commands {
        if cmd.name.is_empty() {
            report.errors.push("Command has empty name".to_string());
        }

        if cmd.binary.is_empty() {
            report
                .errors
                .push(format!("Command '{}' has empty binary path", cmd.name));
        } else {
            let bin_path = path.join(&cmd.binary);
            if !bin_path.exists() {
                let has_build = manifest.hooks.build.is_some();
                if has_build {
                    report.warnings.push(format!(
                        "Command '{}' binary '{}' not found (may be created by build hook)",
                        cmd.name, cmd.binary
                    ));
                } else {
                    report.errors.push(format!(
                        "Command '{}' binary '{}' not found and no build hook defined",
                        cmd.name, cmd.binary
                    ));
                }
            }
        }
    }

    if let Some(ref build) = manifest.hooks.build {
        if build.trim().is_empty() {
            report
                .warnings
                .push("hooks.build is set but empty".to_string());
        }
    }

    print_plugin_report(&report, strict, json)
}

fn print_plugin_report(report: &PluginValidationReport, strict: bool, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else if report.errors.is_empty() && report.warnings.is_empty() {
        let name = if report.plugin_name.is_empty() {
            &report.path
        } else {
            &report.plugin_name
        };
        println!(
            "{} {} — valid",
            style("✅").green().bold(),
            style(name).green()
        );
    } else {
        let name = if report.plugin_name.is_empty() {
            &report.path
        } else {
            &report.plugin_name
        };
        println!("{}", style(name).bold());
        for e in &report.errors {
            println!("  {} {}", style("error:").red().bold(), e);
        }
        for w in &report.warnings {
            println!("  {} {}", style("warn:").yellow().bold(), w);
        }
    }

    let has_errors = !report.errors.is_empty();
    let has_warnings = !report.warnings.is_empty();
    if has_errors || (strict && has_warnings) {
        bail!("Validation failed");
    }

    Ok(())
}

fn publish_plugin(
    path: &Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let yes = yes || json || crate::utils::is_non_interactive();
    let config = crate::config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;

    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;

    let manifest_path = path.join("plugin.toml");
    validate_plugin(&path, false, false)?;

    let content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let manifest: PluginManifest = toml::from_str(&content).context("Invalid plugin.toml")?;

    let repo_name = &manifest.plugin.name;
    let desc = description
        .or(manifest.plugin.description.as_deref())
        .unwrap_or("A fledge plugin");

    let owner = match org {
        Some(o) => o.to_string(),
        None => crate::publish::get_authenticated_user(&token)?,
    };

    if !json {
        println!(
            "{} Publishing plugin {} as {}/{}",
            style("➡️").cyan().bold(),
            style(path.display()).dim(),
            style(&owner).green(),
            style(repo_name).green()
        );
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Checking repository:"))
    };
    let repo_exists = crate::publish::check_repo_exists(&owner, repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    if repo_exists {
        if !yes {
            crate::utils::require_interactive("yes")?;
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(format!(
                        "Repository {}/{} already exists. Push update?",
                        owner, repo_name
                    ))
                    .default(false)
                    .interact()?;

            if !confirm {
                if !json {
                    println!("{} Cancelled.", style("*").cyan().bold());
                }
                return Ok(());
            }
        }
    } else {
        let sp = if json {
            None
        } else {
            Some(crate::spinner::Spinner::start("Creating repository:"))
        };
        crate::publish::create_github_repo(repo_name, desc, private, org, &token)?;
        if let Some(s) = sp {
            s.finish();
        }
        if !json {
            println!(
                "  {} Created repository {}/{}",
                style("✅").green().bold(),
                owner,
                repo_name
            );
        }
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Setting repository topics:"))
    };
    crate::publish::set_repo_topic(&owner, repo_name, "fledge-plugin", &token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style("fledge-plugin").cyan()
        );
    }

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Pushing plugin files:"))
    };
    crate::publish::push_directory(&path, &owner, repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !json {
        println!("  {} Pushed plugin files", style("✅").green().bold());
    }

    let visibility = if private { "private" } else { "public" };
    if json {
        let result = serde_json::json!({
            "schema_version": 1,
            "action": "publish",
            "repo": {
                "owner": owner,
                "name": repo_name,
                "url": format!("https://github.com/{}/{}", owner, repo_name),
            },
            "visibility": visibility,
            "validated": true,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "\n{} Published! Install with:\n\n  {}",
            style("✅").green().bold(),
            style(format!("fledge plugins install {}/{}", owner, repo_name)).cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_plugin_source_dir_walks_symlink_to_plugin_root() {
        // Mirror the post-install layout:
        //   <root>/plugins/my-plugin/bin/fledge-my-plugin     ← real binary
        //   <root>/plugins/bin/fledge-my-plugin               ← shared symlink
        //
        // resolve_plugin_source_dir(<symlink>) should return
        //   <root>/plugins/my-plugin
        let tmp = tempfile::TempDir::new().unwrap();
        let plugin_root = tmp.path().join("plugins").join("my-plugin");
        let plugin_bin_dir = plugin_root.join("bin");
        std::fs::create_dir_all(&plugin_bin_dir).unwrap();
        let real_binary = plugin_bin_dir.join("fledge-my-plugin");
        std::fs::write(&real_binary, "#!/bin/sh\nexit 0\n").unwrap();

        let shared_bin_dir = tmp.path().join("plugins").join("bin");
        std::fs::create_dir_all(&shared_bin_dir).unwrap();
        let symlink = shared_bin_dir.join("fledge-my-plugin");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_binary, &symlink).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real_binary, &symlink).unwrap();

        let resolved = resolve_plugin_source_dir(&symlink).expect("resolve should succeed");
        // Canonicalize the expected path because TempDir may live under /tmp,
        // which is itself a symlink to /private/tmp on macOS.
        let expected = std::fs::canonicalize(&plugin_root).unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_plugin_source_dir_handles_non_symlink_path() {
        // Direct PATH-resolved fledge-<name> binaries (not installed via
        // `fledge plugins install`) still get a sensible plugin dir — the
        // parent of the parent of the binary.
        let tmp = tempfile::TempDir::new().unwrap();
        let bin_dir = tmp.path().join("project").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("fledge-direct");
        std::fs::write(&bin, "").unwrap();

        let resolved = resolve_plugin_source_dir(&bin).expect("should resolve");
        let expected = std::fs::canonicalize(tmp.path().join("project")).unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn default_plugins_are_well_formed() {
        // Every entry should parse via parse_source_ref so --defaults can't
        // ship a bogus ref that fails at install time.
        assert!(!DEFAULT_PLUGINS.is_empty());
        for src in DEFAULT_PLUGINS {
            let (owner_repo, _git_ref) = parse_source_ref(src);
            assert!(
                owner_repo.contains('/'),
                "DEFAULT_PLUGINS entry '{src}' must be owner/repo"
            );
            // No accidental @ref pinning in the default list — defaults
            // should track the latest stable, not be frozen on a tag.
            assert!(
                !src.contains('@'),
                "DEFAULT_PLUGINS entry '{src}' should not pin a ref"
            );
        }
    }

    #[test]
    fn install_action_rejects_source_with_defaults() {
        let err = install_action(Some("someone/foo"), false, true, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--defaults"));
    }

    #[test]
    fn install_action_requires_source_or_defaults() {
        let err = install_action(None, false, false, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--defaults"));
    }

    #[test]
    fn update_plugins_rejects_name_with_defaults() {
        let err = update_plugins(Some("foo"), true, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--defaults"));
    }

    #[test]
    fn default_source_match_recognizes_shorthand() {
        // The matcher used by `update_plugins(.., defaults=true)` must
        // recognize stored sources in any of three forms: bare shorthand,
        // normalized URL, or URL without `.git`. This duplicates the
        // closure in `update_plugins` to test the matching logic in
        // isolation — keep the two in sync.
        let is_default = |source: &str| -> bool {
            DEFAULT_PLUGINS.iter().any(|d| {
                source == *d
                    || source == normalize_source(d)
                    || source.trim_end_matches(".git").ends_with(d)
            })
        };
        assert!(is_default("CorvidLabs/fledge-plugin-github"));
        assert!(is_default(
            "https://github.com/CorvidLabs/fledge-plugin-github.git"
        ));
        assert!(is_default(
            "https://github.com/CorvidLabs/fledge-plugin-github"
        ));
        assert!(!is_default("CorvidLabs/fledge-plugin-figma"));
        assert!(!is_default("someone/random-plugin"));
    }

    #[test]
    fn normalize_github_shorthand() {
        assert_eq!(
            normalize_source("someone/fledge-deploy"),
            "https://github.com/someone/fledge-deploy.git"
        );
    }

    #[test]
    fn normalize_github_shorthand_with_ref() {
        assert_eq!(
            normalize_source("someone/fledge-deploy@v1.0.0"),
            "https://github.com/someone/fledge-deploy.git"
        );
    }

    #[test]
    fn normalize_full_url() {
        let url = "https://github.com/someone/fledge-deploy.git";
        assert_eq!(normalize_source(url), url);
    }

    #[test]
    fn normalize_ssh_url() {
        let url = "git@github.com:someone/fledge-deploy.git";
        assert_eq!(normalize_source(url), url);
    }

    #[test]
    fn extract_name_from_github_shorthand() {
        assert_eq!(
            extract_name_from_source("someone/fledge-deploy"),
            "fledge-deploy"
        );
    }

    #[test]
    fn extract_name_with_ref() {
        assert_eq!(
            extract_name_from_source("someone/fledge-deploy@v1.0.0"),
            "fledge-deploy"
        );
    }

    #[test]
    fn extract_name_from_full_url() {
        assert_eq!(
            extract_name_from_source("https://github.com/someone/fledge-deploy.git"),
            "fledge-deploy"
        );
    }

    #[test]
    fn extract_name_plain() {
        assert_eq!(extract_name_from_source("my-plugin"), "my-plugin");
    }

    #[test]
    fn plugin_dir_is_under_config() {
        let dir = plugins_dir();
        assert!(dir.to_string_lossy().contains("fledge"));
        assert!(dir.to_string_lossy().contains("plugins"));
    }

    #[test]
    fn bin_dir_is_under_plugins() {
        let dir = plugin_bin_dir();
        assert!(dir.ends_with("plugins/bin"));
    }

    #[test]
    fn empty_registry_has_no_plugins() {
        let registry = PluginsRegistry {
            plugins: Vec::new(),
        };
        assert!(registry.plugins.is_empty());
    }

    #[test]
    fn registry_roundtrip() {
        let registry = PluginsRegistry {
            plugins: vec![PluginEntry {
                name: "fledge-test".to_string(),
                source: "someone/fledge-test".to_string(),
                version: "1.0.0".to_string(),
                installed: "2026-04-20".to_string(),
                commands: vec!["test-cmd".to_string()],
                pinned_ref: None,
                capabilities: None,
            }],
        };
        let serialized = toml::to_string_pretty(&registry).unwrap();
        let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.plugins.len(), 1);
        assert_eq!(deserialized.plugins[0].name, "fledge-test");
        assert_eq!(deserialized.plugins[0].commands, vec!["test-cmd"]);
        assert!(deserialized.plugins[0].pinned_ref.is_none());
        assert!(deserialized.plugins[0].capabilities.is_none());
    }

    #[test]
    fn registry_roundtrip_with_pinned_ref() {
        let registry = PluginsRegistry {
            plugins: vec![PluginEntry {
                name: "fledge-test".to_string(),
                source: "someone/fledge-test".to_string(),
                version: "1.0.0".to_string(),
                installed: "2026-04-20".to_string(),
                commands: vec!["test-cmd".to_string()],
                pinned_ref: Some("v1.0.0".to_string()),
                capabilities: None,
            }],
        };
        let serialized = toml::to_string_pretty(&registry).unwrap();
        let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.plugins[0].pinned_ref,
            Some("v1.0.0".to_string())
        );
    }

    #[test]
    fn registry_roundtrip_with_capabilities() {
        let registry = PluginsRegistry {
            plugins: vec![PluginEntry {
                name: "fledge-deploy".to_string(),
                source: "someone/fledge-deploy".to_string(),
                version: "1.0.0".to_string(),
                installed: "2026-04-22".to_string(),
                commands: vec!["deploy".to_string()],
                pinned_ref: None,
                capabilities: Some(PluginCapabilities {
                    exec: true,
                    store: true,
                    metadata: false,
                }),
            }],
        };
        let serialized = toml::to_string_pretty(&registry).unwrap();
        let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
        let caps = deserialized.plugins[0].capabilities.as_ref().unwrap();
        assert!(caps.exec);
        assert!(caps.store);
        assert!(!caps.metadata);
    }

    #[test]
    fn parse_source_ref_with_tag() {
        let (base, git_ref) = parse_source_ref("someone/fledge-deploy@v1.2.0");
        assert_eq!(base, "someone/fledge-deploy");
        assert_eq!(git_ref, Some("v1.2.0"));
    }

    #[test]
    fn parse_source_ref_without_tag() {
        let (base, git_ref) = parse_source_ref("someone/fledge-deploy");
        assert_eq!(base, "someone/fledge-deploy");
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_source_ref_with_branch() {
        let (base, git_ref) = parse_source_ref("someone/fledge-deploy@main");
        assert_eq!(base, "someone/fledge-deploy");
        assert_eq!(git_ref, Some("main"));
    }

    #[test]
    fn parse_source_ref_full_url_with_tag() {
        let (base, git_ref) =
            parse_source_ref("https://github.com/someone/fledge-deploy.git@v2.0.0");
        assert_eq!(base, "https://github.com/someone/fledge-deploy.git");
        assert_eq!(git_ref, Some("v2.0.0"));
    }

    #[test]
    fn parse_source_ref_credential_url_no_split() {
        let (base, git_ref) = parse_source_ref("https://user:token@github.com/owner/repo.git");
        assert_eq!(base, "https://user:token@github.com/owner/repo.git");
        assert!(git_ref.is_none());
    }

    #[test]
    fn validate_plugin_name_rejects_dotdot() {
        assert!(validate_plugin_name("..").is_err());
    }

    #[test]
    fn validate_plugin_name_rejects_hidden() {
        assert!(validate_plugin_name(".secret").is_err());
    }

    #[test]
    fn validate_plugin_name_rejects_slashes() {
        assert!(validate_plugin_name("../etc").is_err());
    }

    #[test]
    fn validate_plugin_name_accepts_normal() {
        assert!(validate_plugin_name("fledge-deploy").is_ok());
    }

    #[test]
    fn validate_command_name_rejects_slashes() {
        assert!(validate_command_name("../evil").is_err());
        assert!(validate_command_name("foo/bar").is_err());
    }

    #[test]
    fn validate_command_name_rejects_dot_prefix() {
        assert!(validate_command_name(".hidden").is_err());
    }

    #[test]
    fn validate_command_name_rejects_dash_prefix() {
        assert!(validate_command_name("-flag").is_err());
    }

    #[test]
    fn validate_command_name_accepts_normal() {
        assert!(validate_command_name("deploy").is_ok());
        assert!(validate_command_name("my-tool").is_ok());
        assert!(validate_command_name("tool_v2").is_ok());
    }

    #[test]
    fn parse_plugin_manifest() {
        let manifest_str = r#"
[plugin]
name = "fledge-deploy"
version = "0.1.0"
description = "Deploy to cloud"
author = "someone"

[[commands]]
name = "deploy"
description = "Deploy the project"
binary = "fledge-deploy"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert_eq!(manifest.plugin.name, "fledge-deploy");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert_eq!(manifest.commands.len(), 1);
        assert_eq!(manifest.commands[0].name, "deploy");
    }

    #[test]
    fn parse_minimal_manifest() {
        let manifest_str = r#"
[plugin]
name = "fledge-minimal"
version = "0.1.0"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert_eq!(manifest.plugin.name, "fledge-minimal");
        assert!(manifest.commands.is_empty());
        assert!(!manifest.capabilities.exec);
        assert!(!manifest.capabilities.store);
        assert!(!manifest.capabilities.metadata);
    }

    #[test]
    fn parse_manifest_with_capabilities() {
        let manifest_str = r#"
[plugin]
name = "fledge-deploy"
version = "0.1.0"
protocol = "fledge-v1"

[capabilities]
exec = true
store = true
metadata = false

[[commands]]
name = "deploy"
binary = "fledge-deploy"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert!(manifest.capabilities.exec);
        assert!(manifest.capabilities.store);
        assert!(!manifest.capabilities.metadata);
    }

    #[test]
    fn parse_manifest_partial_capabilities() {
        let manifest_str = r#"
[plugin]
name = "fledge-stats"
version = "0.1.0"
protocol = "fledge-v1"

[capabilities]
store = true
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert!(!manifest.capabilities.exec);
        assert!(manifest.capabilities.store);
        assert!(!manifest.capabilities.metadata);
    }

    #[test]
    fn parse_manifest_multiple_commands() {
        let manifest_str = r#"
[plugin]
name = "fledge-cloud"
version = "0.2.0"

[[commands]]
name = "deploy"
description = "Deploy"
binary = "bin/deploy"

[[commands]]
name = "rollback"
description = "Rollback"
binary = "bin/rollback"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert_eq!(manifest.commands.len(), 2);
        assert_eq!(manifest.commands[0].name, "deploy");
        assert_eq!(manifest.commands[1].name, "rollback");
    }

    #[test]
    fn resolve_nonexistent_plugin() {
        assert!(resolve_plugin_command("definitely-not-installed-xyz").is_none());
    }

    #[test]
    fn which_nonexistent() {
        assert!(which_fledge_plugin("definitely-not-installed-xyz").is_none());
    }

    #[test]
    fn install_dir_with_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin_dir = tmp.path().join("test-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let manifest = r#"
[plugin]
name = "test-plugin"
version = "0.1.0"
"#;
        fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();

        let content = fs::read_to_string(plugin_dir.join("plugin.toml")).unwrap();
        let parsed: PluginManifest = toml::from_str(&content).unwrap();
        assert_eq!(parsed.plugin.name, "test-plugin");
    }

    #[test]
    fn registry_path_exists() {
        let path = registry_path();
        assert!(path.to_string_lossy().contains("plugins.toml"));
    }

    #[test]
    fn plugins_dir_structure() {
        let pd = plugins_dir();
        let bd = plugin_bin_dir();
        assert!(bd.starts_with(&pd));
    }

    #[test]
    fn detect_rust_build() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();
        let result = detect_build_command(tmp.path());
        assert!(result.is_some());
        let (lang, cmd) = result.unwrap();
        assert_eq!(lang, "Rust");
        assert_eq!(cmd[0], "cargo");
    }

    #[test]
    fn detect_swift_build() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Package.swift"), "// swift").unwrap();
        let result = detect_build_command(tmp.path());
        assert!(result.is_some());
        let (lang, _) = result.unwrap();
        assert_eq!(lang, "Swift");
    }

    #[test]
    fn detect_go_build() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("go.mod"), "module x").unwrap();
        let result = detect_build_command(tmp.path());
        assert!(result.is_some());
        let (lang, _) = result.unwrap();
        assert_eq!(lang, "Go");
    }

    #[test]
    fn detect_node_build() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let result = detect_build_command(tmp.path());
        assert!(result.is_some());
        let (lang, _) = result.unwrap();
        assert_eq!(lang, "Node");
    }

    #[test]
    fn detect_no_build_system() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(detect_build_command(tmp.path()).is_none());
    }

    #[test]
    fn parse_manifest_with_build_hook() {
        let manifest_str = r#"
[plugin]
name = "fledge-compiled"
version = "0.1.0"

[[commands]]
name = "compiled"
binary = "target/release/fledge-compiled"

[hooks]
build = "cargo build --release"
post_install = "scripts/setup.sh"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert_eq!(
            manifest.hooks.build.as_deref(),
            Some("cargo build --release")
        );
        assert_eq!(
            manifest.hooks.post_install.as_deref(),
            Some("scripts/setup.sh")
        );
    }

    #[test]
    fn parse_manifest_with_lifecycle_hooks() {
        let manifest_str = r#"
[plugin]
name = "fledge-lint"
version = "0.1.0"

[hooks]
pre_init = "scripts/pre-init.sh"
post_work_start = "scripts/setup-hooks.sh"
pre_pr = "scripts/lint-all.sh"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert_eq!(
            manifest.hooks.pre_init.as_deref(),
            Some("scripts/pre-init.sh")
        );
        assert_eq!(
            manifest.hooks.post_work_start.as_deref(),
            Some("scripts/setup-hooks.sh")
        );
        assert_eq!(
            manifest.hooks.pre_pr.as_deref(),
            Some("scripts/lint-all.sh")
        );
    }

    #[test]
    fn parse_manifest_lifecycle_hooks_default_none() {
        let manifest_str = r#"
[plugin]
name = "fledge-simple"
version = "0.1.0"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert!(manifest.hooks.pre_init.is_none());
        assert!(manifest.hooks.post_work_start.is_none());
        assert!(manifest.hooks.pre_pr.is_none());
    }

    #[test]
    fn create_plugin_scaffolds_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        create_plugin("my-plugin", tmp.path(), Some("Test plugin"), true, false).unwrap();

        let target = tmp.path().join("my-plugin");
        assert!(target.join("plugin.toml").exists());
        assert!(target.join("README.md").exists());
        assert!(target.join(".gitignore").exists());
        assert!(target.join("bin").is_dir());
        assert!(target.join("bin/my-plugin").exists());

        let content = fs::read_to_string(target.join("plugin.toml")).unwrap();
        let manifest: PluginManifest = toml::from_str(&content).unwrap();
        assert_eq!(manifest.plugin.name, "my-plugin");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert_eq!(manifest.commands.len(), 1);
    }

    #[test]
    fn create_plugin_fails_if_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("existing")).unwrap();
        let result = create_plugin("existing", tmp.path(), None, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn validate_valid_plugin() {
        let tmp = tempfile::TempDir::new().unwrap();
        create_plugin("test-plugin", tmp.path(), Some("Test"), true, false).unwrap();

        let result = validate_plugin(&tmp.path().join("test-plugin"), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_missing_plugin_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = validate_plugin(tmp.path(), false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No plugin.toml"));
    }

    #[test]
    fn validate_empty_name_is_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(
            tmp.path().join("plugin.toml"),
            r#"
[plugin]
name = ""
version = "0.1.0"
"#,
        )
        .unwrap();

        let result = validate_plugin(tmp.path(), false, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_missing_binary_is_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(
            tmp.path().join("plugin.toml"),
            r#"
[plugin]
name = "test"
version = "0.1.0"

[[commands]]
name = "test"
description = "Test"
binary = "bin/nonexistent"
"#,
        )
        .unwrap();

        let result = validate_plugin(tmp.path(), false, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_missing_binary_with_build_hook_is_warning() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(
            tmp.path().join("plugin.toml"),
            r#"
[plugin]
name = "test"
version = "0.1.0"
description = "Test"
author = "tester"

[[commands]]
name = "test"
description = "Test"
binary = "target/release/test"

[hooks]
build = "cargo build --release"
"#,
        )
        .unwrap();

        // non-strict: passes with warning
        let result = validate_plugin(tmp.path(), false, false);
        assert!(result.is_ok());

        // strict: fails on warning
        let result = validate_plugin(tmp.path(), true, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_json_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        create_plugin("json-test", tmp.path(), Some("Test"), true, false).unwrap();

        let result = validate_plugin(&tmp.path().join("json-test"), false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn trust_tier_official_github_shorthand() {
        assert_eq!(
            determine_trust_tier("CorvidLabs/fledge-plugin-deploy"),
            TrustTier::Official
        );
    }

    #[test]
    fn trust_tier_official_full_url() {
        assert_eq!(
            determine_trust_tier("https://github.com/CorvidLabs/fledge-plugin-deploy.git"),
            TrustTier::Official
        );
    }

    #[test]
    fn trust_tier_official_ssh_url() {
        assert_eq!(
            determine_trust_tier("git@github.com:CorvidLabs/fledge-plugin-deploy.git"),
            TrustTier::Official
        );
    }

    #[test]
    fn trust_tier_official_with_ref() {
        assert_eq!(
            determine_trust_tier("CorvidLabs/fledge-plugin-deploy@v1.0.0"),
            TrustTier::Official
        );
    }

    #[test]
    fn trust_tier_official_lowercase() {
        assert_eq!(
            determine_trust_tier("corvidlabs/fledge-plugin-deploy"),
            TrustTier::Official
        );
    }

    #[test]
    fn trust_tier_unverified_third_party() {
        assert_eq!(
            determine_trust_tier("someone/fledge-plugin-cool"),
            TrustTier::Unverified
        );
    }

    #[test]
    fn trust_tier_unverified_full_url() {
        assert_eq!(
            determine_trust_tier("https://github.com/random-user/fledge-deploy.git"),
            TrustTier::Unverified
        );
    }

    #[test]
    fn trust_tier_unverified_no_org() {
        assert_eq!(determine_trust_tier("local-plugin"), TrustTier::Unverified);
    }

    #[test]
    fn trust_tier_label_strings() {
        assert_eq!(TrustTier::Official.label(), "official");
        assert_eq!(TrustTier::Team.label(), "team");
        assert_eq!(TrustTier::Unverified.label(), "unverified");
    }
}
