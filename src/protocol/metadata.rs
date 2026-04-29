use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

pub(crate) fn handle_metadata(keys: &[String]) -> Result<serde_json::Value> {
    let mut result = serde_json::Map::new();

    for key in keys {
        match key.as_str() {
            "fledge_config" => {
                let config_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("fledge.toml");
                if config_path.exists() {
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        if let Ok(parsed) = content.parse::<toml::Value>() {
                            result.insert(
                                key.clone(),
                                serde_json::to_value(parsed).unwrap_or(serde_json::Value::Null),
                            );
                            continue;
                        }
                    }
                }
                result.insert(key.clone(), serde_json::Value::Null);
            }
            "git_tags" => {
                let tags: Vec<String> = Command::new("git")
                    .args(["tag", "--sort=-v:refname", "--no-column"])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .take(100)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key.clone(),
                    serde_json::to_value(tags).unwrap_or(serde_json::Value::Null),
                );
            }
            "git_status" => {
                let files: Vec<String> = Command::new("git")
                    .args(["status", "--porcelain"])
                    .output()
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key.clone(),
                    serde_json::to_value(files).unwrap_or(serde_json::Value::Null),
                );
            }
            "git_log" => {
                let entries: Vec<String> = Command::new("git")
                    .args(["log", "--oneline", "-20"])
                    .output()
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key.clone(),
                    serde_json::to_value(entries).unwrap_or(serde_json::Value::Null),
                );
            }
            "env" => {
                let sensitive_patterns = [
                    "secret",
                    "token",
                    "password",
                    "key",
                    "credential",
                    "auth",
                    "private",
                    "session",
                    "cookie",
                ];
                let dangerous_prefixes = ["ld_preload", "ld_library_path", "dyld_", "kubeconfig"];
                let safe_vars: HashMap<String, String> = std::env::vars()
                    .filter(|(k, v)| {
                        let lower = k.to_lowercase();
                        let is_sensitive_name =
                            sensitive_patterns.iter().any(|p| lower.contains(p));
                        let is_dangerous_prefix =
                            dangerous_prefixes.iter().any(|p| lower.starts_with(p));
                        let looks_like_conn_string = lower.ends_with("_url")
                            || lower.ends_with("_uri")
                            || lower.ends_with("_dsn");
                        let value_has_creds = v.contains('@') && v.contains(':');
                        !is_sensitive_name
                            && !is_dangerous_prefix
                            && !looks_like_conn_string
                            && !value_has_creds
                    })
                    .collect();
                result.insert(
                    key.clone(),
                    serde_json::to_value(safe_vars).unwrap_or(serde_json::Value::Null),
                );
            }
            _ => {
                result.insert(key.clone(), serde_json::Value::Null);
            }
        }
    }

    Ok(serde_json::Value::Object(result))
}
