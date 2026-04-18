use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub templates: TemplatesConfig,
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
}

impl Config {
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
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("fledge")
            .join("config.toml")
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
}
