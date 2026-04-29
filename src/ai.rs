use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::Config;
use crate::llm::ProviderKind;
use crate::utils;

/// Per-command JSON schema versions for `ai` subcommands. See lanes.rs for
/// rationale.
const AI_STATUS_SCHEMA: u32 = 1;
const AI_MODELS_SCHEMA: u32 = 1;

/// CLI actions dispatched from `fledge ai`.
pub enum AiAction {
    Status {
        json: bool,
    },
    Models {
        provider: Option<String>,
        search: Option<String>,
        json: bool,
    },
    Use {
        provider: Option<String>,
        model: Option<String>,
    },
}

pub fn run(action: AiAction) -> Result<()> {
    match action {
        AiAction::Status { json } => status(json),
        AiAction::Models {
            provider,
            search,
            json,
        } => models(provider, search, json),
        AiAction::Use { provider, model } => use_provider(provider, model),
    }
}

/// Where a resolved value came from, so `fledge ai status` can explain itself.
#[derive(Debug, Serialize)]
enum Source {
    #[serde(rename = "env")]
    Env,
    #[serde(rename = "config")]
    ConfigFile,
    #[serde(rename = "default")]
    Default,
}

impl Source {
    fn label(&self) -> &'static str {
        match self {
            Source::Env => "env",
            Source::ConfigFile => "config",
            Source::Default => "default",
        }
    }
}

#[derive(Debug, Serialize)]
struct StatusReport {
    provider: String,
    provider_source: Source,
    model: Option<String>,
    model_source: Option<Source>,
    host: Option<String>,
    host_source: Option<Source>,
}

fn status(json: bool) -> Result<()> {
    let config = Config::load().context("loading config")?;
    let (kind, provider_source) = resolve_provider_with_source(&config)?;

    let (model, model_source, host, host_source) = match kind {
        ProviderKind::Claude => {
            let (m, s) = resolve_claude_model(&config);
            (m, s, None, None)
        }
        ProviderKind::Ollama => {
            let (m, ms) = resolve_ollama_model(&config);
            let (h, hs) = resolve_ollama_host(&config);
            (Some(m), Some(ms), Some(h), Some(hs))
        }
    };

    let report = StatusReport {
        provider: kind.as_str().to_string(),
        provider_source,
        model,
        model_source,
        host,
        host_source,
    };

    if json {
        let envelope = serde_json::json!({
            "schema_version": AI_STATUS_SCHEMA,
            "action": "ai_status",
            "provider": report.provider,
            "provider_source": report.provider_source,
            "model": report.model,
            "model_source": report.model_source,
            "host": report.host,
            "host_source": report.host_source,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    println!(
        "  {} {} {}",
        style("Provider:").bold(),
        style(&report.provider).cyan(),
        style(format!("(from {})", report.provider_source.label())).dim()
    );
    match (&report.model, &report.model_source) {
        (Some(m), Some(src)) => println!(
            "     {} {} {}",
            style("Model:").bold(),
            style(m).cyan(),
            style(format!("(from {})", src.label())).dim()
        ),
        _ => println!(
            "     {} {}",
            style("Model:").bold(),
            style("(provider default)").dim()
        ),
    }
    if let (Some(h), Some(src)) = (&report.host, &report.host_source) {
        println!(
            "      {} {} {}",
            style("Host:").bold(),
            style(h).cyan(),
            style(format!("(from {})", src.label())).dim()
        );
    }
    Ok(())
}

fn resolve_provider_with_source(config: &Config) -> Result<(ProviderKind, Source)> {
    if let Ok(v) = std::env::var("FLEDGE_AI_PROVIDER") {
        return Ok((ProviderKind::parse(&v)?, Source::Env));
    }
    if let Some(v) = &config.ai.provider {
        return Ok((ProviderKind::parse(v)?, Source::ConfigFile));
    }
    Ok((ProviderKind::Claude, Source::Default))
}

fn resolve_claude_model(config: &Config) -> (Option<String>, Option<Source>) {
    if let Ok(v) = std::env::var("FLEDGE_AI_MODEL") {
        return (Some(v), Some(Source::Env));
    }
    if let Some(v) = &config.ai.claude.model {
        return (Some(v.clone()), Some(Source::ConfigFile));
    }
    (None, None)
}

fn resolve_ollama_model(config: &Config) -> (String, Source) {
    if let Ok(v) = std::env::var("FLEDGE_AI_MODEL") {
        return (v, Source::Env);
    }
    // config.ai.ollama.model has a serde default, so we can't cleanly tell
    // "user set this" vs "default kicked in" — report default when it equals
    // the built-in string; otherwise config.
    let default_model = "llama3.3";
    if config.ai.ollama.model == default_model {
        (default_model.to_string(), Source::Default)
    } else {
        (config.ai.ollama.model.clone(), Source::ConfigFile)
    }
}

fn resolve_ollama_host(config: &Config) -> (String, Source) {
    if let Ok(v) = std::env::var("OLLAMA_HOST") {
        return (crate::llm::normalize_ollama_host(&v), Source::Env);
    }
    let normalized = crate::llm::normalize_ollama_host(&config.ai.ollama.host);
    let default_host = "http://localhost:11434";
    if normalized == default_host {
        (default_host.to_string(), Source::Default)
    } else {
        (normalized, Source::ConfigFile)
    }
}

// ---- models ---------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTagModel>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
struct OllamaTagModel {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    details: Option<OllamaTagDetails>,
    #[serde(default)]
    remote_host: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
struct OllamaTagDetails {
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    parameter_size: Option<String>,
    #[serde(default)]
    quantization_level: Option<String>,
}

/// Curated, intentionally short list of Claude aliases agents commonly reach
/// for. Claude CLI accepts arbitrary aliases, so this is guidance — not the
/// authoritative catalog.
const CLAUDE_WELL_KNOWN_MODELS: &[&str] = &["opus-4.7", "opus-4.6", "sonnet-4.6", "haiku-4.5"];

#[derive(Debug, Serialize)]
struct ModelEntry {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameter_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_host: Option<String>,
}

fn models(provider: Option<String>, search: Option<String>, json: bool) -> Result<()> {
    let config = Config::load().context("loading config")?;
    let kind = match provider {
        Some(ref p) => ProviderKind::parse(p)?,
        None => resolve_provider_with_source(&config)?.0,
    };

    let entries = match kind {
        ProviderKind::Ollama => list_ollama_models(&config)?,
        ProviderKind::Claude => CLAUDE_WELL_KNOWN_MODELS
            .iter()
            .map(|n| ModelEntry {
                name: (*n).to_string(),
                family: None,
                parameter_size: None,
                quantization: None,
                size_bytes: None,
                remote_host: None,
            })
            .collect(),
    };

    let filtered: Vec<ModelEntry> = match search {
        Some(q) if !q.is_empty() => {
            let q = q.to_ascii_lowercase();
            entries
                .into_iter()
                .filter(|m| m.name.to_ascii_lowercase().contains(&q))
                .collect()
        }
        _ => entries,
    };

    if json {
        let envelope = serde_json::json!({
            "schema_version": AI_MODELS_SCHEMA,
            "action": "ai_models",
            "provider": kind.as_str(),
            "models": filtered,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    if filtered.is_empty() {
        println!("  No models found.");
        if matches!(kind, ProviderKind::Claude) {
            println!(
                "  {}",
                style("(claude models aren't live-enumerable — pass any alias to --model)").dim()
            );
        }
        return Ok(());
    }

    let noun = if filtered.len() == 1 {
        "model"
    } else {
        "models"
    };
    println!(
        "  {} {noun} for {}:",
        filtered.len(),
        style(kind.as_str()).cyan()
    );
    for m in &filtered {
        let mut line = format!("    {}", style(&m.name).bold());
        let mut details: Vec<String> = Vec::new();
        // Ollama's /api/tags returns empty strings rather than nulls for
        // some cloud models; filter those out so we don't render `[, , ,]`.
        if let Some(p) = m.parameter_size.as_deref().filter(|s| !s.is_empty()) {
            details.push(p.to_string());
        }
        if let Some(f) = m.family.as_deref().filter(|s| !s.is_empty()) {
            details.push(f.to_string());
        }
        if let Some(q) = m.quantization.as_deref().filter(|s| !s.is_empty()) {
            details.push(q.to_string());
        }
        if let Some(host) = m.remote_host.as_deref().filter(|s| !s.is_empty()) {
            details.push(format!("cloud → {host}"));
        }
        if !details.is_empty() {
            line.push_str(&format!(
                "  {}",
                style(format!("[{}]", details.join(", "))).dim()
            ));
        }
        println!("{line}");
    }
    if matches!(kind, ProviderKind::Claude) {
        println!(
            "\n  {}",
            style("(curated list — pass any alias to --model; claude CLI won't validate ahead of time)")
                .dim()
        );
    }
    Ok(())
}

fn list_ollama_models(config: &Config) -> Result<Vec<ModelEntry>> {
    let host = crate::llm::normalize_ollama_host(
        &std::env::var("OLLAMA_HOST").unwrap_or_else(|_| config.ai.ollama.host.clone()),
    );
    let url = format!("{}/api/tags", host.trim_end_matches('/'));

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();

    let mut req = ureq::Agent::get(&agent, &url).header("User-Agent", "fledge-cli");
    if let Some(key) = std::env::var("OLLAMA_API_KEY")
        .ok()
        .or_else(|| config.ai.ollama.api_key.clone())
    {
        req = req.header("Authorization", &format!("Bearer {key}"));
    }

    let result = req.call();
    let mut response = match result {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(code)) => {
            anyhow::bail!(
                "Ollama endpoint returned HTTP {code} from {url}. Check the host and API key."
            );
        }
        Err(e) => {
            return Err(anyhow::Error::new(e))
                .with_context(|| format!("GET {url} (is the Ollama server running?)"));
        }
    };

    let body = response
        .body_mut()
        .read_to_string()
        .with_context(|| format!("reading response from {url}"))?;
    let parsed: OllamaTagsResponse =
        serde_json::from_str(&body).with_context(|| format!("decoding response from {url}"))?;

    Ok(parsed
        .models
        .into_iter()
        .map(|m| ModelEntry {
            family: m.details.as_ref().and_then(|d| d.family.clone()),
            parameter_size: m.details.as_ref().and_then(|d| d.parameter_size.clone()),
            quantization: m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone()),
            size_bytes: m.size,
            remote_host: m.remote_host,
            name: m.name,
        })
        .collect())
}

// ---- use ------------------------------------------------------------------

fn use_provider(provider: Option<String>, model: Option<String>) -> Result<()> {
    let mut config = Config::load().context("loading config")?;

    let kind = match provider {
        Some(p) => ProviderKind::parse(&p)?,
        None => {
            utils::require_interactive("provider")?;
            let items = ["claude", "ollama"];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select AI provider")
                .items(&items)
                .default(0)
                .interact()
                .context("reading provider selection")?;
            ProviderKind::parse(items[selection])?
        }
    };

    // Resolve the model. If supplied explicitly, use it. Otherwise prompt
    // (when the model-slot is non-optional, e.g. Ollama) or skip.
    let chosen_model: Option<String> = match model {
        Some(m) => Some(m),
        None => prompt_for_model(kind, &config)?,
    };

    // Persist to config.
    config.set("ai.provider", kind.as_str())?;
    match kind {
        ProviderKind::Claude => {
            if let Some(m) = &chosen_model {
                config.set("ai.claude.model", m)?;
            }
        }
        ProviderKind::Ollama => {
            if let Some(m) = &chosen_model {
                config.set("ai.ollama.model", m)?;
            }
        }
    }
    config.save().context("saving config")?;

    println!(
        "{} Active provider: {}{}",
        style("✅").green().bold(),
        style(kind.as_str()).cyan(),
        match &chosen_model {
            Some(m) => format!(" ({})", style(m).cyan()),
            None => String::new(),
        }
    );
    Ok(())
}

/// Interactive model picker. Returns `None` when the user declines or no
/// interactive session is available and a model wasn't supplied.
fn prompt_for_model(kind: ProviderKind, config: &Config) -> Result<Option<String>> {
    if !utils::is_interactive() {
        // No prompt fallback in non-interactive mode — caller must supply --model.
        return Ok(None);
    }

    match kind {
        ProviderKind::Ollama => {
            // Try to list live models; fall back to free-text input on error.
            let live = list_ollama_models(config).ok();
            let names: Vec<String> = live
                .as_ref()
                .map(|v| v.iter().map(|m| m.name.clone()).collect())
                .unwrap_or_default();

            if names.is_empty() {
                let input: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Model name (e.g. llama3.3, qwen3-coder:480b-cloud)")
                    .allow_empty(true)
                    .interact_text()
                    .context("reading model input")?;
                Ok(if input.is_empty() { None } else { Some(input) })
            } else {
                let mut items: Vec<String> = names.clone();
                items.push("(custom…)".to_string());
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select Ollama model")
                    .items(&items)
                    .default(0)
                    .interact()
                    .context("reading model selection")?;
                if selection == items.len() - 1 {
                    let input: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Model name")
                        .interact_text()
                        .context("reading custom model input")?;
                    Ok(Some(input))
                } else {
                    Ok(Some(names[selection].clone()))
                }
            }
        }
        ProviderKind::Claude => {
            let mut items: Vec<String> = CLAUDE_WELL_KNOWN_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect();
            items.push("(use claude default)".to_string());
            items.push("(custom…)".to_string());
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select Claude model")
                .items(&items)
                .default(0)
                .interact()
                .context("reading model selection")?;
            if selection == items.len() - 2 {
                Ok(None)
            } else if selection == items.len() - 1 {
                let input: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Model alias")
                    .interact_text()
                    .context("reading custom model input")?;
                Ok(Some(input))
            } else {
                Ok(Some(items[selection].clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AiConfig, OllamaConfig};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::Mutex;
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn clear_env() {
        std::env::remove_var("FLEDGE_AI_PROVIDER");
        std::env::remove_var("FLEDGE_AI_MODEL");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OLLAMA_API_KEY");
    }

    #[test]
    fn status_provider_defaults_report_default_source() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let (kind, src) = resolve_provider_with_source(&config).unwrap();
        assert_eq!(kind, ProviderKind::Claude);
        assert!(matches!(src, Source::Default));
    }

    #[test]
    fn status_provider_env_source() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("FLEDGE_AI_PROVIDER", "ollama");
        let config = Config::default();
        let (kind, src) = resolve_provider_with_source(&config).unwrap();
        assert_eq!(kind, ProviderKind::Ollama);
        assert!(matches!(src, Source::Env));
        clear_env();
    }

    #[test]
    fn status_provider_config_source() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ..Default::default()
            },
            ..Config::default()
        };
        let (kind, src) = resolve_provider_with_source(&config).unwrap();
        assert_eq!(kind, ProviderKind::Ollama);
        assert!(matches!(src, Source::ConfigFile));
    }

    #[test]
    fn ollama_host_config_vs_default() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let (host, src) = resolve_ollama_host(&config);
        assert_eq!(host, "http://localhost:11434");
        assert!(matches!(src, Source::Default));

        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    host: "https://ollama.com".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let (host, src) = resolve_ollama_host(&config);
        assert_eq!(host, "https://ollama.com");
        assert!(matches!(src, Source::ConfigFile));
    }

    #[test]
    fn ollama_host_env_wins() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("OLLAMA_HOST", "https://override.example.com");
        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    host: "https://ollama.com".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let (host, src) = resolve_ollama_host(&config);
        assert_eq!(host, "https://override.example.com");
        assert!(matches!(src, Source::Env));
        clear_env();
    }

    #[test]
    fn ollama_model_env_wins_over_config() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("FLEDGE_AI_MODEL", "env-model");
        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    model: "config-model".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let (m, src) = resolve_ollama_model(&config);
        assert_eq!(m, "env-model");
        assert!(matches!(src, Source::Env));
        clear_env();
    }

    #[test]
    fn claude_model_absent_when_unset() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let (m, src) = resolve_claude_model(&config);
        assert!(m.is_none());
        assert!(src.is_none());
    }
}
