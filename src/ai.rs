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
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key_source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cloud_routed: Option<bool>,
}

fn status(json: bool) -> Result<()> {
    let config = Config::load().context("loading config")?;
    let (kind, provider_source) = resolve_provider_with_source(&config)?;

    let (model, model_source, host, host_source, api_key_source, cloud_routed) = match kind {
        ProviderKind::Anthropic => {
            let (m, s) = resolve_anthropic_model(&config);
            let aks = resolve_anthropic_api_key_source(&config);
            let host = config.ai.anthropic.base_url.clone();
            let host_source = host.as_ref().map(|_| Source::ConfigFile);
            (m, s, host, host_source, aks, None)
        }
        ProviderKind::OpenAi => {
            let (m, s) = resolve_openai_model(&config);
            let aks = resolve_openai_api_key_source(&config);
            let host = config.ai.openai.base_url.clone();
            let host_source = host.as_ref().map(|_| Source::ConfigFile);
            (m, s, host, host_source, aks, None)
        }
        ProviderKind::Ollama => {
            let (m, ms) = resolve_ollama_model(&config);
            let aks = resolve_ollama_api_key_source(&config);
            let api_key = std::env::var("OLLAMA_API_KEY")
                .ok()
                .or_else(|| config.ai.ollama.api_key.clone())
                .filter(|k| !k.is_empty());
            let effective_host = crate::llm::resolve_effective_host(&config, &m, &api_key);
            let is_cloud = crate::llm::is_cloud_model(&m);
            let routed_to_cloud = is_cloud
                && api_key.is_some()
                && effective_host == crate::llm::DEFAULT_OLLAMA_CLOUD_HOST;

            let h_source = if std::env::var("OLLAMA_HOST").is_ok() {
                Source::Env
            } else if routed_to_cloud {
                Source::Default
            } else {
                let default_host = "http://localhost:11434";
                if crate::llm::normalize_ollama_host(&config.ai.ollama.host) == default_host {
                    Source::Default
                } else {
                    Source::ConfigFile
                }
            };

            (
                Some(m),
                Some(ms),
                Some(effective_host),
                Some(h_source),
                aks,
                Some(routed_to_cloud),
            )
        }
    };

    let report = StatusReport {
        provider: kind.as_str().to_string(),
        provider_source,
        model,
        model_source,
        host,
        host_source,
        api_key_source,
        cloud_routed,
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
            "api_key_set": report.api_key_source.is_some(),
            "api_key_source": report.api_key_source,
            "cloud_routed": report.cloud_routed,
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
        let cloud_note = if report.cloud_routed == Some(true) {
            " (auto-routed to cloud)".to_string()
        } else {
            format!(" (from {})", src.label())
        };
        println!(
            "      {} {} {}",
            style("Host:").bold(),
            style(h).cyan(),
            style(cloud_note).dim()
        );
    }
    let not_set_hint = match report.provider.as_str() {
        "ollama" => "(not set — required for cloud models)",
        "openai" => {
            "(not set — export OPENAI_API_KEY or run `fledge config set ai.openai.api_key <key>`)"
        }
        _ => {
            "(not set — export ANTHROPIC_API_KEY or run `fledge config set ai.anthropic.api_key <key>`)"
        }
    };
    match &report.api_key_source {
        Some(src) => println!(
            "   {} {} {}",
            style("API Key:").bold(),
            style("***").green(),
            style(format!("(from {})", src.label())).dim()
        ),
        None => println!(
            "   {} {}",
            style("API Key:").bold(),
            style(not_set_hint).dim()
        ),
    }
    Ok(())
}

fn resolve_anthropic_api_key_source(config: &Config) -> Option<Source> {
    if std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .is_some()
    {
        return Some(Source::Env);
    }
    // Prefer the new key, fall back to the deprecated `ai.claude.api_key`.
    let configured = config
        .ai
        .anthropic
        .api_key
        .as_ref()
        .or(config.ai.claude.api_key.as_ref())
        .filter(|k| !k.is_empty());
    if configured.is_some() {
        return Some(Source::ConfigFile);
    }
    None
}

fn resolve_openai_api_key_source(config: &Config) -> Option<Source> {
    if std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .is_some()
    {
        return Some(Source::Env);
    }
    if config
        .ai
        .openai
        .api_key
        .as_ref()
        .filter(|k| !k.is_empty())
        .is_some()
    {
        return Some(Source::ConfigFile);
    }
    None
}

fn resolve_provider_with_source(config: &Config) -> Result<(ProviderKind, Source)> {
    if let Ok(v) = std::env::var("FLEDGE_AI_PROVIDER") {
        return Ok((ProviderKind::parse(&v)?, Source::Env));
    }
    if let Some(v) = &config.ai.provider {
        return Ok((ProviderKind::parse(v)?, Source::ConfigFile));
    }
    Ok((ProviderKind::Ollama, Source::Default))
}

fn resolve_anthropic_model(config: &Config) -> (Option<String>, Option<Source>) {
    if let Ok(v) = std::env::var("FLEDGE_AI_MODEL") {
        return (Some(v), Some(Source::Env));
    }
    // Prefer the new key, fall back to the deprecated `ai.claude.model`.
    if let Some(v) = config
        .ai
        .anthropic
        .model
        .as_ref()
        .or(config.ai.claude.model.as_ref())
    {
        return (Some(v.clone()), Some(Source::ConfigFile));
    }
    (None, None)
}

fn resolve_openai_model(config: &Config) -> (Option<String>, Option<Source>) {
    if let Ok(v) = std::env::var("FLEDGE_AI_MODEL") {
        return (Some(v), Some(Source::Env));
    }
    if let Some(v) = &config.ai.openai.model {
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

fn resolve_ollama_api_key_source(config: &Config) -> Option<Source> {
    if std::env::var("OLLAMA_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .is_some()
    {
        return Some(Source::Env);
    }
    if config
        .ai
        .ollama
        .api_key
        .as_ref()
        .filter(|k| !k.is_empty())
        .is_some()
    {
        return Some(Source::ConfigFile);
    }
    None
}

#[cfg(test)]
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

/// Curated, intentionally short list of Anthropic model ids agents commonly
/// reach for. The Messages API accepts the exact id, so this is guidance, not
/// the authoritative catalog.
const ANTHROPIC_WELL_KNOWN_MODELS: &[&str] =
    &["claude-opus-4-8", "claude-sonnet-4-6", "claude-haiku-4-5"];

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
        ProviderKind::Anthropic => ANTHROPIC_WELL_KNOWN_MODELS
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
        // OpenAI-compatible endpoints aren't uniformly enumerable; the user
        // picks a model id for whatever gateway base_url points at.
        ProviderKind::OpenAi => Vec::new(),
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
        if matches!(kind, ProviderKind::Anthropic | ProviderKind::OpenAi) {
            println!(
                "  {}",
                style("(API models aren't live-enumerable — pass any model id to --model)").dim()
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
    if matches!(kind, ProviderKind::Anthropic) {
        println!(
            "\n  {}",
            style("(curated list — pass any model id to --model)").dim()
        );
    }
    Ok(())
}

fn list_ollama_models(config: &Config) -> Result<Vec<ModelEntry>> {
    let api_key = std::env::var("OLLAMA_API_KEY")
        .ok()
        .or_else(|| config.ai.ollama.api_key.clone())
        .filter(|k| !k.is_empty());

    let host = crate::llm::normalize_ollama_host(
        &std::env::var("OLLAMA_HOST").unwrap_or_else(|_| config.ai.ollama.host.clone()),
    );
    let url = format!("{}/api/tags", host.trim_end_matches('/'));

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();

    let mut req = ureq::Agent::get(&agent, &url).header("User-Agent", "fledge-cli");
    if let Some(ref key) = api_key {
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
            let items = ["anthropic", "openai", "ollama"];
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

    // If a cloud model is selected and no API key is configured, prompt for one.
    if let Some(ref m) = chosen_model {
        if matches!(kind, ProviderKind::Ollama)
            && crate::llm::is_cloud_model(m)
            && resolve_ollama_api_key_source(&config).is_none()
            && utils::is_interactive()
        {
            eprintln!(
                "  {} Cloud model '{}' requires an API key.",
                style("⚠").yellow().bold(),
                style(m).cyan()
            );
            eprintln!(
                "  Get one at {} or run {}",
                style("https://ollama.com/settings/keys").underlined(),
                style("ollama signin").cyan()
            );
            let key: String = dialoguer::Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Ollama API key (empty to skip)")
                .allow_empty_password(true)
                .interact()
                .context("reading API key")?;
            if !key.is_empty() {
                config.set("ai.ollama.api_key", &key)?;
            }
        }
    }

    // Persist to config.
    config.set("ai.provider", kind.as_str())?;
    match kind {
        ProviderKind::Anthropic => {
            if let Some(m) = &chosen_model {
                config.set("ai.anthropic.model", m)?;
            }
        }
        ProviderKind::OpenAi => {
            if let Some(m) = &chosen_model {
                config.set("ai.openai.model", m)?;
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
        ProviderKind::Anthropic => {
            let mut items: Vec<String> = ANTHROPIC_WELL_KNOWN_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect();
            items.push("(use default)".to_string());
            items.push("(custom…)".to_string());
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select Anthropic model")
                .items(&items)
                .default(0)
                .interact()
                .context("reading model selection")?;
            if selection == items.len() - 2 {
                Ok(None)
            } else if selection == items.len() - 1 {
                let input: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Model id")
                    .interact_text()
                    .context("reading custom model input")?;
                Ok(Some(input))
            } else {
                Ok(Some(items[selection].clone()))
            }
        }
        ProviderKind::OpenAi => {
            // No uniform catalog across OpenAI-compatible gateways; free-text.
            let input: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Model id (e.g. gpt-4o, anthropic/claude-sonnet-4-6)")
                .allow_empty(true)
                .interact_text()
                .context("reading model input")?;
            Ok(if input.is_empty() { None } else { Some(input) })
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
        assert_eq!(kind, ProviderKind::Ollama);
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
    fn anthropic_model_absent_when_unset() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let (m, src) = resolve_anthropic_model(&config);
        assert!(m.is_none());
        assert!(src.is_none());
    }
}
