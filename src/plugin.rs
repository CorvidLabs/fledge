use anyhow::{Context, Result, bail};
use console::style;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct PluginManifest {
    plugin: PluginMeta,
    #[serde(default)]
    commands: Vec<PluginCommand>,
    #[serde(default)]
    hooks: Vec<PluginHook>,
}

#[derive(Debug, Deserialize)]
struct PluginMeta {
    name: String,
    version: String,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    author: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PluginCommand {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    binary: String,
}

#[derive(Debug, Deserialize)]
struct PluginHook {
    #[allow(dead_code)]
    event: String,
    #[allow(dead_code)]
    binary: String,
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
}

pub struct PluginOptions {
    pub action: PluginAction,
    pub json: bool,
}

pub enum PluginAction {
    Install { source: String, force: bool },
    Remove { name: String },
    List,
    Search { query: Option<String>, limit: usize },
    Run { name: String, args: Vec<String> },
}

pub fn run(opts: PluginOptions) -> Result<()> {
    match opts.action {
        PluginAction::Install { source, force } => install_plugin(&source, force),
        PluginAction::Remove { name } => remove_plugin(&name),
        PluginAction::List => list_plugins(opts.json),
        PluginAction::Search { query, limit } => search_plugins(query.as_deref(), limit, opts.json),
        PluginAction::Run { name, args } => run_plugin(&name, &args),
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

#[allow(dead_code)]
pub fn list_installed() -> Result<Vec<PluginEntry>> {
    let registry = load_registry()?;
    Ok(registry.plugins)
}

fn plugins_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("fledge")
        .join("plugins")
}

fn plugin_bin_dir() -> PathBuf {
    plugins_dir().join("bin")
}

fn registry_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
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
    if source.starts_with("https://") || source.starts_with("git@") {
        source.to_string()
    } else if source.contains('/') {
        format!("https://github.com/{}.git", source)
    } else {
        source.to_string()
    }
}

fn extract_name_from_source(source: &str) -> String {
    source
        .rsplit('/')
        .next()
        .unwrap_or(source)
        .trim_end_matches(".git")
        .to_string()
}

fn install_plugin(source: &str, force: bool) -> Result<()> {
    let url = normalize_source(source);
    let repo_name = extract_name_from_source(source);

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

    println!(
        "  {} Cloning {}...",
        style("▸").cyan().bold(),
        style(&url).dim()
    );

    let status = Command::new("git")
        .args(["clone", "--depth", "1", &url])
        .arg(&plugin_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("running git clone")?;

    if !status.success() {
        bail!(
            "Failed to clone '{}'. Check the repository URL and your network connection.",
            source
        );
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

    let mut command_names = Vec::new();
    for cmd in &manifest.commands {
        let binary_path = plugin_dir.join(&cmd.binary);
        if !binary_path.exists() {
            fs::remove_dir_all(&plugin_dir).ok();
            bail!(
                "Plugin '{}' references binary '{}' which does not exist in the repository.",
                manifest.plugin.name,
                cmd.binary
            );
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

    let entry = PluginEntry {
        name: repo_name.clone(),
        source: source.to_string(),
        version: manifest.plugin.version.clone(),
        installed: chrono::Local::now().format("%Y-%m-%d").to_string(),
        commands: command_names.clone(),
    };

    if let Some(idx) = existing {
        registry.plugins[idx] = entry;
    } else {
        registry.plugins.push(entry);
    }
    save_registry(&registry)?;

    println!(
        "{} Installed {} v{}",
        style("✓").green().bold(),
        style(&manifest.plugin.name).green(),
        manifest.plugin.version
    );
    if !command_names.is_empty() {
        println!("  Commands: {}", style(command_names.join(", ")).cyan());
    }
    if !manifest.hooks.is_empty() {
        println!("  Hooks: {} registered", style(manifest.hooks.len()).cyan());
    }

    Ok(())
}

fn remove_plugin(name: &str) -> Result<()> {
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
    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir).context("removing plugin directory")?;
    }

    let removed_name = entry.name.clone();
    registry.plugins.remove(idx);
    save_registry(&registry)?;

    println!(
        "{} Removed {}",
        style("✓").green().bold(),
        style(&removed_name).green()
    );

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
                serde_json::json!({
                    "name": p.name,
                    "version": p.version,
                    "source": p.source,
                    "installed": p.installed,
                    "commands": p.commands,
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
        println!(
            "  {:<width$}  {}  {}",
            style(&plugin.name).green(),
            style(format!("v{}", plugin.version)).dim(),
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

fn search_plugins(query: Option<&str>, limit: usize, json: bool) -> Result<()> {
    let search_query = match query {
        Some(q) => format!("fledge-plugin {q}"),
        None => "fledge-plugin".to_string(),
    };

    println!(
        "  {} Searching GitHub for plugins...",
        style("▸").cyan().bold()
    );

    let config = crate::config::Config::load().ok();
    let token = config.as_ref().and_then(|c| c.github_token());

    let query_str = format!("{search_query} topic:fledge-plugin");
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
                serde_json::json!({
                    "name": item["name"],
                    "full_name": item["full_name"],
                    "description": item["description"],
                    "stars": item["stargazers_count"],
                    "url": item["html_url"],
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
        let desc = item["description"].as_str().unwrap_or("(no description)");
        let stars = item["stargazers_count"].as_u64().unwrap_or(0);
        println!(
            "  {:<width$}  {}  {}",
            style(full_name).green(),
            style(desc).dim(),
            style(format!("★ {stars}")).yellow(),
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
    let bin_path = resolve_plugin_command(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Plugin command '{}' not found.\n  Run {} to see installed plugins.",
            name,
            style("fledge plugin list").cyan()
        )
    })?;

    let status = Command::new(&bin_path)
        .args(args)
        .status()
        .with_context(|| format!("running plugin '{name}'"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Plugin '{}' exited with code {}", name, code);
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_github_shorthand() {
        assert_eq!(
            normalize_source("someone/fledge-deploy"),
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
            }],
        };
        let serialized = toml::to_string_pretty(&registry).unwrap();
        let deserialized: PluginsRegistry = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.plugins.len(), 1);
        assert_eq!(deserialized.plugins[0].name, "fledge-test");
        assert_eq!(deserialized.plugins[0].commands, vec!["test-cmd"]);
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

[[hooks]]
event = "lane:post"
binary = "fledge-deploy-notify"
"#;
        let manifest: PluginManifest = toml::from_str(manifest_str).unwrap();
        assert_eq!(manifest.plugin.name, "fledge-deploy");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert_eq!(manifest.commands.len(), 1);
        assert_eq!(manifest.commands[0].name, "deploy");
        assert_eq!(manifest.hooks.len(), 1);
        assert_eq!(manifest.hooks[0].event, "lane:post");
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
        assert!(manifest.hooks.is_empty());
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
}
