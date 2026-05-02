use anyhow::{bail, Context, Result};
use console::style;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::trust::parse_source_ref;

mod create;
mod install;
mod list;
mod publish;
mod remove;
mod run_plugin;
mod search;
mod update;
mod validate;

#[cfg(test)]
mod tests;

// ─── Internal imports for the run() dispatcher ─────────────────────────────

use create::create_plugin;
use install::install_action;
use list::{audit_plugins, list_plugins};
use publish::publish_plugin;
use remove::remove_plugin;
use run_plugin::{run_hook, run_plugin_cmd};
use search::search_plugins;
use update::update_plugins;
use validate::validate_plugin;

#[cfg(test)]
use run_plugin::{apply_protocol, resolve_plugin_source_dir, which_fledge_plugin};

// ─── Constants ───────────────────────────────────────────────────────────────

/// The curated set of plugins fledge endorses as "the default install."
///
/// These are the plugins that took over commands removed from core in
/// v0.15 (the tight-core refactor). Running `fledge plugins install --defaults`
/// installs all of them so a fresh fledge install gets back to feature
/// parity with v0.14 in one command.
///
/// Every entry is pinned to a release tag (`owner/repo@vX.Y.Z`).
/// Bump the tag here when adopting a new plugin release.
pub const DEFAULT_PLUGINS: &[&str] = &[
    "CorvidLabs/fledge-plugin-github@v0.4.0",
    "CorvidLabs/fledge-plugin-deps@v0.1.0",
    "CorvidLabs/fledge-plugin-metrics@v0.2.0",
];

/// Per-command JSON schema versions. Each constant tracks the wire shape of one
/// `plugins` subcommand's `--json` envelope independently so that future shape
/// changes can bump exactly the affected envelope without semantically
/// corrupting the meaning of `schema_version` for unrelated commands. Additive
/// changes (new optional fields) do not bump.
const PLUGINS_INSTALL_SCHEMA: u32 = 1;
const PLUGINS_UPDATE_SCHEMA: u32 = 1;
const PLUGINS_REMOVE_SCHEMA: u32 = 1;
const PLUGINS_LIST_SCHEMA: u32 = 1;
const PLUGINS_AUDIT_SCHEMA: u32 = 1;
const PLUGINS_SEARCH_SCHEMA: u32 = 1;
const PLUGINS_CREATE_SCHEMA: u32 = 1;
const PLUGINS_PUBLISH_SCHEMA: u32 = 1;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PluginManifest {
    pub(super) plugin: PluginMeta,
    #[serde(default)]
    pub(super) commands: Vec<PluginCommand>,
    #[serde(default)]
    pub(super) hooks: PluginHooks,
    #[serde(default)]
    pub(super) capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub exec: bool,
    #[serde(default)]
    pub store: bool,
    #[serde(default)]
    pub metadata: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<String>,
    #[serde(default)]
    pub network: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PluginMeta {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) description: Option<String>,
    pub(super) author: Option<String>,
    pub(super) protocol: Option<String>,
    #[serde(default)]
    pub(super) runtime: Option<String>,
}

impl PluginMeta {
    #[allow(dead_code)]
    fn is_wasm(&self) -> bool {
        self.runtime.as_deref() == Some("wasm")
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PluginCommand {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) binary: String,
}

#[derive(Debug, Deserialize, Default)]
struct PluginHooks {
    pub(super) build: Option<String>,
    pub(super) post_install: Option<String>,
    pub(super) post_remove: Option<String>,
    pub(super) pre_init: Option<String>,
    pub(super) post_work_start: Option<String>,
    pub(super) pre_push: Option<String>,
}

impl PluginHooks {
    fn has_any(&self) -> bool {
        self.build.is_some()
            || self.post_install.is_some()
            || self.post_remove.is_some()
            || self.pre_init.is_some()
            || self.post_work_start.is_some()
            || self.pre_push.is_some()
    }

    fn iter_defined(&self) -> Vec<(&str, &str)> {
        let mut hooks = Vec::new();
        if let Some(ref c) = self.pre_push {
            hooks.push(("pre_push", c.as_str()));
        }
        if let Some(ref c) = self.build {
            hooks.push(("build", c.as_str()));
        }
        if let Some(ref c) = self.post_install {
            hooks.push(("post_install", c.as_str()));
        }
        if let Some(ref c) = self.post_remove {
            hooks.push(("post_remove", c.as_str()));
        }
        if let Some(ref c) = self.pre_init {
            hooks.push(("pre_init", c.as_str()));
        }
        if let Some(ref c) = self.post_work_start {
            hooks.push(("post_work_start", c.as_str()));
        }
        hooks
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PluginsRegistry {
    #[serde(default)]
    pub(super) plugins: Vec<PluginEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PluginEntry {
    pub(super) name: String,
    pub(super) source: String,
    pub(super) version: String,
    pub(super) installed: String,
    #[serde(default)]
    pub(super) commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) pinned_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) capabilities: Option<PluginCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) runtime: Option<String>,
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
    },
    Create {
        name: String,
        output: PathBuf,
        description: Option<String>,
        yes: bool,
    },
    Validate {
        path: PathBuf,
        strict: bool,
        json: bool,
    },
}

// ─── Dispatcher ──────────────────────────────────────────────────────────────

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
        PluginAction::Run { name, args } => run_plugin_cmd(&name, &args),
        PluginAction::Publish {
            path,
            org,
            private,
            description,
            yes,
        } => publish_plugin(
            &path,
            org.as_deref(),
            private,
            description.as_deref(),
            yes,
            opts.json,
        ),
        PluginAction::Create {
            name,
            output,
            description,
            yes,
        } => create_plugin(&name, &output, description.as_deref(), yes, opts.json),
        PluginAction::Validate { path, strict, json } => validate_plugin(&path, strict, json),
    }
}

pub fn run_lifecycle_hook(event: &str) -> Result<()> {
    let registry = load_registry()?;
    for entry in &registry.plugins {
        // Require explicit exec = true; plugins without capabilities declared cannot run hooks
        match &entry.capabilities {
            Some(caps) if caps.exec => {}
            _ => continue,
        }

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
            "pre_push" => &manifest.hooks.pre_push,
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

pub fn resolve_plugin_command(name: &str) -> Option<PathBuf> {
    let bin_dir = plugin_bin_dir();
    let bin_path = bin_dir.join(format!("fledge-{name}"));
    if bin_path.exists() {
        return Some(bin_path);
    }
    run_plugin::which_fledge_plugin(name)
}

// ─── Registry helpers ────────────────────────────────────────────────────────

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

// ─── Source helpers ──────────────────────────────────────────────────────────

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

// ─── Build helpers ───────────────────────────────────────────────────────────

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

// ─── Security helpers ─────────────────────────────────────────────────────────

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

// ─── Git auth helper ─────────────────────────────────────────────────────────

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

// ─── Platform helpers ────────────────────────────────────────────────────────

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

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
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
