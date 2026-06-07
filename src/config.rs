use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub templates: TemplatesConfig,
    #[serde(default)]
    pub github: GitHubConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub trust: TrustConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TrustConfig {
    #[serde(default)]
    pub orgs: Vec<String>,
    #[serde(default)]
    pub users: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Defaults {
    pub author: Option<String>,
    pub github_org: Option<String>,
    pub license: Option<String>,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            author: None,
            github_org: None,
            license: Some("MIT".to_string()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TemplatesConfig {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub repos: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub token: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AiConfig {
    /// Active provider: `anthropic`, `openai`, or `ollama`. `None` defaults to
    /// `ollama` (works locally with no key). `claude` is a deprecated alias of
    /// `anthropic`.
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "AnthropicConfig::is_empty")]
    pub anthropic: AnthropicConfig,
    #[serde(default, skip_serializing_if = "OpenAiConfig::is_empty")]
    pub openai: OpenAiConfig,
    /// Deprecated alias of `anthropic`. Still read so existing configs keep
    /// working; removed in fledge 2.0.
    #[serde(default, skip_serializing_if = "ClaudeConfig::is_empty")]
    pub claude: ClaudeConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
}

/// Native Anthropic provider config, served over HTTP by the `corvid-ai` crate.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Model id (e.g. `claude-sonnet-4-6`). When None, the crate default applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// API key. Read from `ANTHROPIC_API_KEY` env var when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Endpoint base URL override (default `https://api.anthropic.com`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl AnthropicConfig {
    fn is_empty(&self) -> bool {
        self.model.is_none() && self.api_key.is_none() && self.base_url.is_none()
    }
}

/// Generic OpenAI-compatible provider config (OpenAI, OpenRouter, Groq, Together,
/// DeepSeek, Mistral, xAI, local servers, ...). The gateway is chosen by
/// `base_url`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// Endpoint base URL (e.g. `https://openrouter.ai/api/v1`). Default is OpenAI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// API key. Read from `OPENAI_API_KEY` env var when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model id. Required: OpenAI-compatible endpoints have no built-in default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl OpenAiConfig {
    fn is_empty(&self) -> bool {
        self.base_url.is_none() && self.api_key.is_none() && self.model.is_none()
    }
}

/// Deprecated alias of [`AnthropicConfig`]. Read as a fallback for existing
/// `ai.claude.*` configs; removed in fledge 2.0.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClaudeConfig {
    /// Deprecated. Use `ai.anthropic.model`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Deprecated. Use `ai.anthropic.api_key` or `ANTHROPIC_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl ClaudeConfig {
    fn is_empty(&self) -> bool {
        self.model.is_none() && self.api_key.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Base URL of the Ollama-compatible endpoint. Skipped during serialization
    /// when it equals the default so `fledge config unset ai.ollama.host`
    /// actually removes the field from the file (issue #377).
    #[serde(
        default = "default_ollama_host",
        skip_serializing_if = "is_default_ollama_host"
    )]
    pub host: String,
    /// Bearer token for Ollama Cloud / Turbo or any authenticated endpoint.
    /// Read from `OLLAMA_API_KEY` env var if this field is None.
    pub api_key: Option<String>,
    /// Default model name (e.g. `llama3.3:70b` or a cloud-registry model).
    #[serde(
        default = "default_ollama_model",
        skip_serializing_if = "is_default_ollama_model"
    )]
    pub model: String,
    /// Per-request timeout in seconds. `FLEDGE_AI_TIMEOUT` env var takes
    /// precedence when set.
    #[serde(
        default = "default_ollama_timeout_seconds",
        skip_serializing_if = "is_default_ollama_timeout_seconds"
    )]
    pub timeout_seconds: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: default_ollama_host(),
            api_key: None,
            model: default_ollama_model(),
            timeout_seconds: default_ollama_timeout_seconds(),
        }
    }
}

fn default_ollama_host() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama3.3".to_string()
}

fn default_ollama_timeout_seconds() -> u64 {
    600
}

fn is_default_ollama_host(host: &str) -> bool {
    host == default_ollama_host()
}

fn is_default_ollama_model(model: &str) -> bool {
    model == default_ollama_model()
}

fn is_default_ollama_timeout_seconds(secs: &u64) -> bool {
    *secs == default_ollama_timeout_seconds()
}

const VALID_KEYS_HINT: &str = "Valid keys: defaults.author, defaults.github_org, defaults.license, github.token, templates.paths, templates.repos, trust.orgs, trust.users, ai.provider, ai.anthropic.model, ai.anthropic.api_key, ai.anthropic.base_url, ai.openai.model, ai.openai.api_key, ai.openai.base_url, ai.ollama.host, ai.ollama.api_key, ai.ollama.model, ai.ollama.timeout_seconds";

impl Config {
    pub fn valid_keys_hint() -> &'static str {
        VALID_KEYS_HINT
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn config_path() -> PathBuf {
        if let Ok(dir) = std::env::var("FLEDGE_CONFIG_DIR") {
            return PathBuf::from(dir).join("config.toml");
        }
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("fledge")
            .join("config.toml")
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            use std::os::unix::fs::PermissionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)?;
            file.write_all(content.as_bytes())?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&path, content)?;
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "defaults.author" => self.defaults.author.clone(),
            "defaults.github_org" => self.defaults.github_org.clone(),
            "defaults.license" => self.defaults.license.clone(),
            "github.token" => self.github.token.clone(),
            "templates.paths" => Some(self.templates.paths.join("\n")),
            "templates.repos" => Some(self.templates.repos.join("\n")),
            "ai.provider" => self.ai.provider.clone(),
            "ai.anthropic.model" => self.ai.anthropic.model.clone(),
            "ai.anthropic.api_key" => self.ai.anthropic.api_key.clone(),
            "ai.anthropic.base_url" => self.ai.anthropic.base_url.clone(),
            "ai.openai.model" => self.ai.openai.model.clone(),
            "ai.openai.api_key" => self.ai.openai.api_key.clone(),
            "ai.openai.base_url" => self.ai.openai.base_url.clone(),
            "ai.claude.model" => self.ai.claude.model.clone(),
            "ai.claude.api_key" => self.ai.claude.api_key.clone(),
            "ai.ollama.host" => Some(self.ai.ollama.host.clone()),
            "ai.ollama.api_key" => self.ai.ollama.api_key.clone(),
            "ai.ollama.model" => Some(self.ai.ollama.model.clone()),
            "ai.ollama.timeout_seconds" => Some(self.ai.ollama.timeout_seconds.to_string()),
            "trust.orgs" => Some(self.trust.orgs.join("\n")),
            "trust.users" => Some(self.trust.users.join("\n")),
            _ => None,
        }
    }

    pub fn is_secret_key(key: &str) -> bool {
        matches!(
            key,
            "github.token"
                | "ai.anthropic.api_key"
                | "ai.openai.api_key"
                | "ai.claude.api_key"
                | "ai.ollama.api_key"
        )
    }

    pub fn is_valid_key(key: &str) -> bool {
        matches!(
            key,
            "defaults.author"
                | "defaults.github_org"
                | "defaults.license"
                | "github.token"
                | "templates.paths"
                | "templates.repos"
                | "trust.orgs"
                | "trust.users"
                | "ai.provider"
                | "ai.anthropic.model"
                | "ai.anthropic.api_key"
                | "ai.anthropic.base_url"
                | "ai.openai.model"
                | "ai.openai.api_key"
                | "ai.openai.base_url"
                | "ai.claude.model"
                | "ai.claude.api_key"
                | "ai.ollama.host"
                | "ai.ollama.api_key"
                | "ai.ollama.model"
                | "ai.ollama.timeout_seconds"
        )
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "defaults.author" => self.defaults.author = Some(value.to_string()),
            "defaults.github_org" => self.defaults.github_org = Some(value.to_string()),
            "defaults.license" => self.defaults.license = Some(value.to_string()),
            "github.token" => self.github.token = Some(value.to_string()),
            "ai.provider" => {
                let normalized = value.trim().to_ascii_lowercase();
                if !matches!(
                    normalized.as_str(),
                    "anthropic" | "openai" | "ollama" | "claude"
                ) {
                    anyhow::bail!(
                        "Invalid provider '{}'. Supported: anthropic, openai, ollama",
                        value
                    );
                }
                self.ai.provider = Some(normalized);
            }
            "ai.anthropic.model" => self.ai.anthropic.model = Some(value.to_string()),
            "ai.anthropic.api_key" => self.ai.anthropic.api_key = Some(value.to_string()),
            "ai.anthropic.base_url" => self.ai.anthropic.base_url = Some(value.to_string()),
            "ai.openai.model" => self.ai.openai.model = Some(value.to_string()),
            "ai.openai.api_key" => self.ai.openai.api_key = Some(value.to_string()),
            "ai.openai.base_url" => self.ai.openai.base_url = Some(value.to_string()),
            "ai.claude.model" => self.ai.claude.model = Some(value.to_string()),
            "ai.claude.api_key" => self.ai.claude.api_key = Some(value.to_string()),
            "ai.ollama.host" => self.ai.ollama.host = value.to_string(),
            "ai.ollama.api_key" => self.ai.ollama.api_key = Some(value.to_string()),
            "ai.ollama.model" => self.ai.ollama.model = value.to_string(),
            "ai.ollama.timeout_seconds" => {
                let secs: u64 = value.trim().parse().map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid timeout '{}' — must be a non-negative integer in seconds",
                        value
                    )
                })?;
                self.ai.ollama.timeout_seconds = secs;
            }
            "templates.paths" | "templates.repos" | "trust.orgs" | "trust.users" => {
                anyhow::bail!(
                    "'{}' is a list key — use `fledge config add/remove {}` instead",
                    key,
                    key
                )
            }
            _ => anyhow::bail!("Unknown config key '{}'. {}", key, VALID_KEYS_HINT),
        }
        Ok(())
    }

    pub fn unset(&mut self, key: &str) -> Result<()> {
        match key {
            "defaults.author" => self.defaults.author = None,
            "defaults.github_org" => self.defaults.github_org = None,
            "defaults.license" => self.defaults.license = None,
            "github.token" => self.github.token = None,
            "templates.paths" => self.templates.paths.clear(),
            "templates.repos" => self.templates.repos.clear(),
            "trust.orgs" => self.trust.orgs.clear(),
            "trust.users" => self.trust.users.clear(),
            "ai.provider" => self.ai.provider = None,
            "ai.anthropic.model" => self.ai.anthropic.model = None,
            "ai.anthropic.api_key" => self.ai.anthropic.api_key = None,
            "ai.anthropic.base_url" => self.ai.anthropic.base_url = None,
            "ai.openai.model" => self.ai.openai.model = None,
            "ai.openai.api_key" => self.ai.openai.api_key = None,
            "ai.openai.base_url" => self.ai.openai.base_url = None,
            "ai.claude.model" => self.ai.claude.model = None,
            "ai.claude.api_key" => self.ai.claude.api_key = None,
            "ai.ollama.host" => self.ai.ollama.host = default_ollama_host(),
            "ai.ollama.api_key" => self.ai.ollama.api_key = None,
            "ai.ollama.model" => self.ai.ollama.model = default_ollama_model(),
            "ai.ollama.timeout_seconds" => {
                self.ai.ollama.timeout_seconds = default_ollama_timeout_seconds()
            }
            _ => anyhow::bail!("Unknown config key '{}'. {}", key, VALID_KEYS_HINT),
        }
        Ok(())
    }

    pub fn add_to_list(&mut self, key: &str, value: &str) -> Result<()> {
        let list = match key {
            "templates.paths" => &mut self.templates.paths,
            "templates.repos" => &mut self.templates.repos,
            "trust.orgs" => &mut self.trust.orgs,
            "trust.users" => &mut self.trust.users,
            key if Self::is_valid_key(key) => {
                anyhow::bail!(
                    "'{}' is a scalar key — use `fledge config set {} <value>` instead",
                    key,
                    key
                )
            }
            _ => anyhow::bail!(
                "Unknown config key '{}'. List keys: templates.paths, templates.repos, trust.orgs, trust.users",
                key
            ),
        };
        let val = value.to_string();
        if !list.contains(&val) {
            list.push(val);
        }
        Ok(())
    }

    pub fn remove_from_list(&mut self, key: &str, value: &str) -> Result<bool> {
        let list = match key {
            "templates.paths" => &mut self.templates.paths,
            "templates.repos" => &mut self.templates.repos,
            "trust.orgs" => &mut self.trust.orgs,
            "trust.users" => &mut self.trust.users,
            key if Self::is_valid_key(key) => {
                anyhow::bail!(
                    "'{}' is a scalar key — use `fledge config unset {}` instead",
                    key,
                    key
                )
            }
            _ => anyhow::bail!(
                "Unknown config key '{}'. List keys: templates.paths, templates.repos, trust.orgs, trust.users",
                key
            ),
        };
        let before = list.len();
        list.retain(|v| v != value);
        Ok(list.len() < before)
    }

    pub fn author_or_git(&self) -> Option<String> {
        if let Some(ref author) = self.defaults.author {
            return Some(author.clone());
        }
        std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                } else {
                    None
                }
            })
    }

    pub fn github_org(&self) -> Option<String> {
        self.defaults.github_org.clone()
    }

    pub fn license(&self) -> String {
        self.defaults
            .license
            .clone()
            .unwrap_or_else(|| "MIT".to_string())
    }

    pub fn github_token(&self) -> Option<String> {
        std::env::var("FLEDGE_GITHUB_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .ok()
            .or_else(|| self.github.token.clone())
            .or_else(|| self.gh_cli_token())
    }

    #[cfg(not(test))]
    fn gh_cli_token(&self) -> Option<String> {
        std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| {
                let s = String::from_utf8(o.stdout).ok()?;
                let s = s.trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            })
    }

    #[cfg(test)]
    fn gh_cli_token(&self) -> Option<String> {
        None
    }

    pub fn template_repos(&self) -> &[String] {
        &self.templates.repos
    }

    pub fn extra_template_paths(&self) -> Vec<PathBuf> {
        self.templates
            .paths
            .iter()
            .map(|p| {
                if let Some(stripped) = p.strip_prefix("~/") {
                    dirs::home_dir().unwrap_or_default().join(stripped)
                } else {
                    PathBuf::from(p)
                }
            })
            .collect()
    }
}

pub fn init_config(preset: Option<&str>) -> Result<()> {
    use console::style;

    let path = Config::config_path();
    if path.exists() {
        anyhow::bail!(
            "Config already exists at {}.\n  Use {} to modify it.",
            style(path.display()).dim(),
            style("fledge config set").cyan()
        );
    }

    let mut config = Config::default();

    if let Some(preset_name) = preset {
        match preset_name {
            "corvidlabs" => {
                config.defaults.author = Some("CorvidLabs".to_string());
                config.defaults.github_org = Some("CorvidLabs".to_string());
                config.defaults.license = Some("MIT".to_string());
                config
                    .templates
                    .repos
                    .push("CorvidLabs/fledge-templates".to_string());
            }
            _ => {
                anyhow::bail!(
                    "Unknown preset '{}'. Available presets: {}",
                    preset_name,
                    style("corvidlabs").cyan()
                );
            }
        }
        config.save()?;
        println!(
            "{} Created config with {} preset at {}",
            style("✅").green().bold(),
            style(preset_name).cyan(),
            style(path.display()).dim()
        );
    } else {
        config.save()?;
        println!(
            "{} Created default config at {}",
            style("✅").green().bold(),
            style(path.display()).dim()
        );
    }

    println!(
        "  Edit with: {}",
        style("fledge config set <key> <value>").cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_mit_license() {
        let config = Config::default();
        assert_eq!(config.license(), "MIT");
    }

    #[test]
    fn default_config_has_no_author() {
        let config = Config::default();
        assert!(config.defaults.author.is_none());
    }

    #[test]
    fn default_config_has_no_github_org() {
        let config = Config::default();
        assert!(config.github_org().is_none());
    }

    #[test]
    fn default_config_has_no_extra_paths() {
        let config = Config::default();
        assert!(config.extra_template_paths().is_empty());
    }

    #[test]
    fn load_from_valid_toml() {
        let toml_str = r#"
[defaults]
author = "Test User"
github_org = "TestOrg"
license = "Apache-2.0"

[templates]
paths = ["/tmp/templates"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.author.as_deref(), Some("Test User"));
        assert_eq!(config.github_org().as_deref(), Some("TestOrg"));
        assert_eq!(config.license(), "Apache-2.0");
        assert_eq!(
            config.extra_template_paths(),
            vec![PathBuf::from("/tmp/templates")]
        );
    }

    #[test]
    fn load_partial_toml_uses_defaults() {
        let toml_str = r#"
[defaults]
author = "Partial"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.author.as_deref(), Some("Partial"));
        assert!(config.github_org().is_none());
        assert_eq!(config.license(), "MIT");
        assert!(config.extra_template_paths().is_empty());
    }

    #[test]
    fn empty_toml_uses_all_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.defaults.author.is_none());
        assert!(config.github_org().is_none());
        assert_eq!(config.license(), "MIT");
    }

    #[test]
    fn license_defaults_when_explicitly_none() {
        let config = Config {
            defaults: Defaults {
                author: None,
                github_org: None,
                license: None,
            },
            ..Config::default()
        };
        assert_eq!(config.license(), "MIT");
    }

    #[test]
    fn extra_paths_expands_tilde() {
        let config = Config {
            templates: TemplatesConfig {
                paths: vec!["~/my-templates".to_string()],
                ..TemplatesConfig::default()
            },
            ..Config::default()
        };
        let paths = config.extra_template_paths();
        assert_eq!(paths.len(), 1);
        assert!(!paths[0].to_string_lossy().contains("~"));
        assert!(paths[0].to_string_lossy().ends_with("my-templates"));
    }

    #[test]
    fn extra_paths_preserves_absolute() {
        let config = Config {
            templates: TemplatesConfig {
                paths: vec!["/opt/templates".to_string()],
                ..TemplatesConfig::default()
            },
            ..Config::default()
        };
        assert_eq!(
            config.extra_template_paths(),
            vec![PathBuf::from("/opt/templates")]
        );
    }

    #[test]
    fn author_or_git_prefers_config() {
        let config = Config {
            defaults: Defaults {
                author: Some("Config Author".to_string()),
                ..Defaults::default()
            },
            ..Config::default()
        };
        assert_eq!(config.author_or_git().as_deref(), Some("Config Author"));
    }

    #[test]
    fn author_or_git_falls_back_to_git() {
        let config = Config::default();
        let result = config.author_or_git();
        // git config user.name may or may not be set — just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result: Result<Config, _> = toml::from_str("not valid [[[toml");
        assert!(result.is_err());
    }

    #[test]
    fn config_path_ends_with_expected_segments() {
        let path = Config::config_path();
        assert!(path.ends_with("fledge/config.toml"));
    }

    #[test]
    fn load_returns_defaults_when_no_file() {
        let config = Config::load().unwrap();
        assert_eq!(config.license(), "MIT");
    }

    #[test]
    fn github_token_from_config_field() {
        let config = Config {
            github: GitHubConfig {
                token: Some("ghp_test123".to_string()),
            },
            ..Config::default()
        };
        // If env vars are set, they take precedence; otherwise config field is used
        let token = config.github_token();
        assert!(token.is_some());
    }

    #[test]
    fn github_config_default_has_no_token() {
        let config = Config::default();
        assert!(config.github.token.is_none());
    }

    #[test]
    fn template_repos_from_config() {
        let toml_str = r#"
[templates]
repos = ["CorvidLabs/fledge-templates", "user/my-templates"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.template_repos().len(), 2);
        assert_eq!(config.template_repos()[0], "CorvidLabs/fledge-templates");
    }

    #[test]
    fn template_repos_default_empty() {
        let config = Config::default();
        assert!(config.template_repos().is_empty());
    }

    #[test]
    fn get_scalar_key() {
        let mut config = Config::default();
        config.defaults.author = Some("Leif".to_string());
        assert_eq!(config.get("defaults.author").as_deref(), Some("Leif"));
    }

    #[test]
    fn get_list_key_returns_newline_separated() {
        let config = Config {
            templates: TemplatesConfig {
                paths: vec!["/a".to_string(), "/b".to_string()],
                ..TemplatesConfig::default()
            },
            ..Config::default()
        };
        assert_eq!(config.get("templates.paths").as_deref(), Some("/a\n/b"));
    }

    #[test]
    fn get_empty_list_returns_empty_string() {
        let config = Config::default();
        assert_eq!(config.get("templates.paths").as_deref(), Some(""));
    }

    #[test]
    fn get_unknown_key_returns_none() {
        let config = Config::default();
        assert!(config.get("nonexistent.key").is_none());
    }

    #[test]
    fn is_valid_key_accepts_all_known_keys() {
        assert!(Config::is_valid_key("defaults.author"));
        assert!(Config::is_valid_key("defaults.github_org"));
        assert!(Config::is_valid_key("defaults.license"));
        assert!(Config::is_valid_key("github.token"));
        assert!(Config::is_valid_key("templates.paths"));
        assert!(Config::is_valid_key("templates.repos"));
    }

    #[test]
    fn is_valid_key_rejects_unknown() {
        assert!(!Config::is_valid_key("unknown.key"));
    }

    #[test]
    fn set_scalar_key() {
        let mut config = Config::default();
        config.set("defaults.author", "Test").unwrap();
        assert_eq!(config.defaults.author.as_deref(), Some("Test"));
    }

    #[test]
    fn set_list_key_errors() {
        let mut config = Config::default();
        assert!(config.set("templates.paths", "/foo").is_err());
    }

    #[test]
    fn set_unknown_key_errors() {
        let mut config = Config::default();
        assert!(config.set("bad.key", "val").is_err());
    }

    #[test]
    fn unset_scalar_key() {
        let mut config = Config::default();
        config.defaults.author = Some("Leif".to_string());
        config.unset("defaults.author").unwrap();
        assert!(config.defaults.author.is_none());
    }

    #[test]
    fn unset_list_key_clears() {
        let mut config = Config {
            templates: TemplatesConfig {
                paths: vec!["/a".to_string()],
                ..TemplatesConfig::default()
            },
            ..Config::default()
        };
        config.unset("templates.paths").unwrap();
        assert!(config.templates.paths.is_empty());
    }

    #[test]
    fn unset_unknown_key_errors() {
        let mut config = Config::default();
        assert!(config.unset("bad.key").is_err());
    }

    #[test]
    fn add_to_list_paths() {
        let mut config = Config::default();
        config.add_to_list("templates.paths", "/my/tpl").unwrap();
        assert_eq!(config.templates.paths, vec!["/my/tpl"]);
    }

    #[test]
    fn add_to_list_repos() {
        let mut config = Config::default();
        config.add_to_list("templates.repos", "user/repo").unwrap();
        assert_eq!(config.templates.repos, vec!["user/repo"]);
    }

    #[test]
    fn add_to_list_deduplicates() {
        let mut config = Config::default();
        config.add_to_list("templates.paths", "/a").unwrap();
        config.add_to_list("templates.paths", "/a").unwrap();
        assert_eq!(config.templates.paths.len(), 1);
    }

    #[test]
    fn add_to_list_scalar_key_errors() {
        let mut config = Config::default();
        assert!(config.add_to_list("defaults.author", "val").is_err());
    }

    #[test]
    fn add_to_list_unknown_key_errors() {
        let mut config = Config::default();
        assert!(config.add_to_list("bad.key", "val").is_err());
    }

    #[test]
    fn remove_from_list_existing() {
        let mut config = Config {
            templates: TemplatesConfig {
                paths: vec!["/a".to_string(), "/b".to_string()],
                ..TemplatesConfig::default()
            },
            ..Config::default()
        };
        let removed = config.remove_from_list("templates.paths", "/a").unwrap();
        assert!(removed);
        assert_eq!(config.templates.paths, vec!["/b"]);
    }

    #[test]
    fn remove_from_list_nonexistent_returns_false() {
        let mut config = Config::default();
        let removed = config.remove_from_list("templates.paths", "/nope").unwrap();
        assert!(!removed);
    }

    #[test]
    fn remove_from_list_scalar_key_errors() {
        let mut config = Config::default();
        assert!(config.remove_from_list("defaults.author", "val").is_err());
    }

    #[test]
    fn ollama_timeout_default_is_600() {
        let config = Config::default();
        assert_eq!(config.ai.ollama.timeout_seconds, 600);
    }

    #[test]
    fn set_ollama_timeout_parses_u64() {
        let mut config = Config::default();
        config.set("ai.ollama.timeout_seconds", "120").unwrap();
        assert_eq!(config.ai.ollama.timeout_seconds, 120);
    }

    #[test]
    fn set_ollama_timeout_rejects_non_integer() {
        let mut config = Config::default();
        let err = config
            .set("ai.ollama.timeout_seconds", "abc")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid timeout"));
    }

    #[test]
    fn unset_ollama_timeout_restores_default() {
        let mut config = Config::default();
        config.set("ai.ollama.timeout_seconds", "7").unwrap();
        config.unset("ai.ollama.timeout_seconds").unwrap();
        assert_eq!(config.ai.ollama.timeout_seconds, 600);
    }

    #[test]
    fn get_ollama_timeout_returns_string() {
        let config = Config::default();
        assert_eq!(
            config.get("ai.ollama.timeout_seconds").as_deref(),
            Some("600")
        );
    }

    #[test]
    fn is_valid_key_accepts_timeout_seconds() {
        assert!(Config::is_valid_key("ai.ollama.timeout_seconds"));
    }

    #[test]
    fn unset_ollama_host_does_not_persist_default() {
        // Issue #377: after unset, the serialized TOML must not contain the
        // hardcoded default host. Equivalent guarantee for model + timeout.
        let mut config = Config::default();
        config
            .set("ai.ollama.host", "https://custom.example.com")
            .unwrap();
        config.set("ai.ollama.model", "gpt-oss:20b").unwrap();
        config.set("ai.ollama.timeout_seconds", "42").unwrap();

        config.unset("ai.ollama.host").unwrap();
        config.unset("ai.ollama.model").unwrap();
        config.unset("ai.ollama.timeout_seconds").unwrap();

        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml.contains("host = \"http://localhost:11434\""),
            "default host should be skipped on serialize:\n{toml}"
        );
        assert!(
            !toml.contains("model = \"llama3.3\""),
            "default model should be skipped on serialize:\n{toml}"
        );
        assert!(
            !toml.contains("timeout_seconds = 600"),
            "default timeout should be skipped on serialize:\n{toml}"
        );
    }

    #[test]
    fn set_then_serialize_keeps_custom_host() {
        let mut config = Config::default();
        config
            .set("ai.ollama.host", "https://custom.example.com")
            .unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(toml.contains("host = \"https://custom.example.com\""));
    }

    #[test]
    fn claude_api_key_is_valid_key() {
        // Issue #379
        assert!(Config::is_valid_key("ai.claude.api_key"));
        assert!(Config::is_secret_key("ai.claude.api_key"));
    }

    #[test]
    fn claude_api_key_set_get_unset() {
        let mut config = Config::default();
        config.set("ai.claude.api_key", "sk-ant-test").unwrap();
        assert_eq!(
            config.get("ai.claude.api_key").as_deref(),
            Some("sk-ant-test")
        );
        config.unset("ai.claude.api_key").unwrap();
        assert!(config.get("ai.claude.api_key").is_none());
    }

    #[test]
    fn corvidlabs_preset_sets_expected_values() {
        let mut config = Config::default();
        config.defaults.author = Some("CorvidLabs".to_string());
        config.defaults.github_org = Some("CorvidLabs".to_string());
        config.defaults.license = Some("MIT".to_string());
        config
            .templates
            .repos
            .push("CorvidLabs/fledge-templates".to_string());

        assert_eq!(config.defaults.author.as_deref(), Some("CorvidLabs"));
        assert_eq!(config.github_org().as_deref(), Some("CorvidLabs"));
        assert_eq!(config.license(), "MIT");
        assert_eq!(config.template_repos(), &["CorvidLabs/fledge-templates"]);
    }

    #[test]
    fn full_config_with_github_section() {
        let toml_str = r#"
[defaults]
author = "Leif"

[github]
token = "ghp_secret"

[templates]
paths = ["/opt/tpl"]
repos = ["CorvidLabs/templates"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.author.as_deref(), Some("Leif"));
        assert_eq!(config.github.token.as_deref(), Some("ghp_secret"));
        assert_eq!(config.template_repos(), &["CorvidLabs/templates"]);
    }

    #[test]
    fn trust_config_defaults_empty() {
        let config = Config::default();
        assert!(config.trust.orgs.is_empty());
        assert!(config.trust.users.is_empty());
    }

    #[test]
    fn trust_config_from_toml() {
        let toml_str = r#"
[trust]
orgs = ["my-company", "other-org"]
users = ["corvid-agent"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.trust.orgs, vec!["my-company", "other-org"]);
        assert_eq!(config.trust.users, vec!["corvid-agent"]);
    }

    #[test]
    fn trust_keys_are_valid() {
        assert!(Config::is_valid_key("trust.orgs"));
        assert!(Config::is_valid_key("trust.users"));
    }

    #[test]
    fn trust_get_returns_newline_separated() {
        let config = Config {
            trust: TrustConfig {
                orgs: vec!["a".to_string(), "b".to_string()],
                users: vec![],
            },
            ..Config::default()
        };
        assert_eq!(config.get("trust.orgs").as_deref(), Some("a\nb"));
    }

    #[test]
    fn trust_set_errors_as_list_key() {
        let mut config = Config::default();
        assert!(config.set("trust.orgs", "val").is_err());
        assert!(config.set("trust.users", "val").is_err());
    }

    #[test]
    fn trust_add_to_list() {
        let mut config = Config::default();
        config.add_to_list("trust.orgs", "my-company").unwrap();
        config.add_to_list("trust.users", "corvid-agent").unwrap();
        assert_eq!(config.trust.orgs, vec!["my-company"]);
        assert_eq!(config.trust.users, vec!["corvid-agent"]);
    }

    #[test]
    fn trust_add_deduplicates() {
        let mut config = Config::default();
        config.add_to_list("trust.orgs", "my-company").unwrap();
        config.add_to_list("trust.orgs", "my-company").unwrap();
        assert_eq!(config.trust.orgs.len(), 1);
    }

    #[test]
    fn trust_remove_from_list() {
        let mut config = Config {
            trust: TrustConfig {
                orgs: vec!["a".to_string(), "b".to_string()],
                users: vec![],
            },
            ..Config::default()
        };
        let removed = config.remove_from_list("trust.orgs", "a").unwrap();
        assert!(removed);
        assert_eq!(config.trust.orgs, vec!["b"]);
    }

    #[test]
    fn trust_unset_clears() {
        let mut config = Config {
            trust: TrustConfig {
                orgs: vec!["a".to_string()],
                users: vec!["b".to_string()],
            },
            ..Config::default()
        };
        config.unset("trust.orgs").unwrap();
        config.unset("trust.users").unwrap();
        assert!(config.trust.orgs.is_empty());
        assert!(config.trust.users.is_empty());
    }

    #[test]
    fn trust_missing_from_toml_defaults_empty() {
        let toml_str = r#"
[defaults]
author = "Leif"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.trust.orgs.is_empty());
        assert!(config.trust.users.is_empty());
    }
}
