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
