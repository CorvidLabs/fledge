use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;

/// Which LLM backend a command should use.
///
/// `Anthropic` and `OpenAi` are served by the `corvid-ai` crate over plain
/// HTTP (Anthropic Messages API and any OpenAI-compatible Chat Completions
/// endpoint). `Ollama` keeps fledge's native client so local/cloud routing and
/// the `/api/tags` model list behave exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
    Ollama,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::OpenAi => "openai",
            ProviderKind::Ollama => "ollama",
        }
    }

    /// Parse a provider name. `claude` is accepted as a deprecated alias for
    /// `anthropic` (the deprecation warning is emitted by `build_provider`, so
    /// this stays pure for status/introspection).
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Ok(ProviderKind::Anthropic),
            "openai" => Ok(ProviderKind::OpenAi),
            "ollama" => Ok(ProviderKind::Ollama),
            other => bail!("Unknown provider '{other}'. Supported: anthropic, openai, ollama"),
        }
    }
}

pub const DEFAULT_OLLAMA_CLOUD_HOST: &str = "https://ollama.com";

/// Returns `true` when the model tag looks like an Ollama Cloud model.
/// Cloud model names use a `-cloud` qualifier (e.g. `qwen3-coder:480b-cloud`).
pub fn is_cloud_model(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.ends_with("-cloud") || lower.contains("-cloud:")
}

/// Resolve the effective Ollama host, accounting for cloud model auto-routing.
///
/// Precedence:
/// 1. `OLLAMA_HOST` env var (explicit user override, always wins)
/// 2. Non-default config host (user explicitly configured a custom host)
/// 3. Cloud host when API key is present AND model is a cloud model
/// 4. Config host (default: `http://localhost:11434`)
pub fn resolve_effective_host(config: &Config, model: &str, api_key: &Option<String>) -> String {
    if let Ok(host) = std::env::var("OLLAMA_HOST") {
        return normalize_ollama_host(&host);
    }

    let config_host = normalize_ollama_host(&config.ai.ollama.host);
    let default_host = "http://localhost:11434";

    if config_host != default_host {
        return config_host;
    }

    if api_key.is_some() && is_cloud_model(model) {
        return DEFAULT_OLLAMA_CLOUD_HOST.to_string();
    }

    config_host
}

/// Returns a parenthetical note when `OLLAMA_HOST` is set, so connection-error
/// messages explain why the request hit the URL it did (issue #378). Returns
/// an empty string when the env var is not set so the call site can append
/// unconditionally.
fn ollama_host_env_hint() -> String {
    match std::env::var("OLLAMA_HOST") {
        Ok(v) if !v.is_empty() => {
            format!(" (OLLAMA_HOST env var = {v}; unset it to use ai.ollama.host config)")
        }
        _ => String::new(),
    }
}

/// Ensure a host string has a scheme; prepend `http://` when missing.
pub fn normalize_ollama_host(host: &str) -> String {
    let h = host.trim().trim_end_matches('/');
    if h.starts_with("http://") || h.starts_with("https://") {
        h.to_string()
    } else {
        format!("http://{h}")
    }
}

/// An invokable LLM.
pub trait LlmProvider: Send + Sync {
    /// Send a prompt, return the model's response as plain text.
    fn invoke(&self, prompt: &str) -> Result<String>;

    /// Human name of the provider (e.g. "anthropic", "ollama").
    fn kind(&self) -> ProviderKind;

    /// The model identifier the provider will use (for display / debug).
    fn model_name(&self) -> Option<&str>;
}

/// Wraps a `corvid-ai` provider (Anthropic native or any OpenAI-compatible
/// endpoint). This is where fledge's `claude` CLI shell-out used to live; it is
/// now a plain HTTP call through the shared crate.
pub struct CorvidProvider {
    inner: corvid_ai::Provider,
    timeout: Duration,
    kind: ProviderKind,
}

impl LlmProvider for CorvidProvider {
    fn invoke(&self, prompt: &str) -> Result<String> {
        self.inner
            .complete(&corvid_ai::Completion::new(prompt), self.timeout)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn kind(&self) -> ProviderKind {
        self.kind
    }

    fn model_name(&self) -> Option<&str> {
        Some(self.inner.model())
    }
}

/// Talks to any Ollama-compatible HTTP endpoint. Works for:
/// - Local Ollama daemon (default `http://localhost:11434`)
/// - Ollama Cloud / Turbo (custom host + `api_key`)
/// - Self-hosted mirrors that speak the same API
pub struct OllamaProvider {
    pub host: String,
    pub api_key: Option<String>,
    pub model: String,
    pub timeout: Duration,
}

impl OllamaProvider {
    fn generate_url(&self) -> String {
        format!("{}/api/generate", self.host.trim_end_matches('/'))
    }
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

impl LlmProvider for OllamaProvider {
    fn invoke(&self, prompt: &str) -> Result<String> {
        let url = self.generate_url();
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
        });
        let body_json = serde_json::to_string(&body).context("encoding Ollama request")?;

        // Generation can legitimately take minutes on large local models;
        // finite timeout prevents silent hangs on a wedged endpoint.
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            .build()
            .into();

        let mut req = ureq::Agent::post(&agent, &url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "fledge-cli");
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", &format!("Bearer {key}"));
        }

        let result = req.send(body_json.as_bytes());
        let mut response = match result {
            Ok(resp) => resp,
            Err(ureq::Error::StatusCode(code)) => {
                bail!(
                    "Ollama endpoint returned HTTP {code} from {url}.{}\n  Check the model name, API key, and host URL.",
                    ollama_host_env_hint()
                );
            }
            Err(e) => {
                return Err(anyhow::Error::new(e)).with_context(|| {
                    format!(
                        "POST {url} (is the Ollama server running?){}",
                        ollama_host_env_hint()
                    )
                });
            }
        };

        let text = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("reading response from {url}"))?;

        let parsed: OllamaGenerateResponse =
            serde_json::from_str(&text).with_context(|| format!("decoding response from {url}"))?;

        Ok(parsed.response.trim().to_string())
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    fn model_name(&self) -> Option<&str> {
        Some(&self.model)
    }
}

/// Resolve the per-request Ollama timeout: `FLEDGE_AI_TIMEOUT` env var
/// (seconds) wins, otherwise `ai.ollama.timeout_seconds` from config
/// (default 600s). Large local models legitimately take minutes; the env
/// var lets users override per-invocation without editing config.
fn resolve_ollama_timeout(config: &Config) -> Duration {
    let secs = std::env::var("FLEDGE_AI_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(config.ai.ollama.timeout_seconds);
    Duration::from_secs(secs)
}

/// Optional `FLEDGE_AI_TIMEOUT` override (seconds) for the corvid-backed
/// providers; `None` lets `corvid-ai` apply its own default.
fn ai_timeout_override() -> Option<u64> {
    std::env::var("FLEDGE_AI_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
}

/// Per-invocation overrides from a CLI flag or programmatic caller. Both
/// fields take precedence over env vars and config.
#[derive(Debug, Default, Clone)]
pub struct ProviderOverride {
    pub provider: Option<String>,
    pub model: Option<String>,
}

/// Resolve the active provider from (in order of precedence):
///   1. explicit override argument
///   2. `FLEDGE_AI_PROVIDER` env var
///   3. `ai.provider` in config
///   4. default: `anthropic`
pub fn resolve_provider_kind(
    config: &Config,
    override_provider: Option<&str>,
) -> Result<ProviderKind> {
    if let Some(v) = override_provider {
        return ProviderKind::parse(v);
    }
    if let Ok(v) = std::env::var("FLEDGE_AI_PROVIDER") {
        return ProviderKind::parse(&v);
    }
    if let Some(v) = &config.ai.provider {
        return ProviderKind::parse(v);
    }
    Ok(ProviderKind::Anthropic)
}

/// The raw provider string the caller selected, in precedence order, so callers
/// can detect the deprecated `claude` alias before it is normalized.
fn selected_provider_string(config: &Config, override_provider: Option<&str>) -> Option<String> {
    override_provider
        .map(str::to_string)
        .or_else(|| std::env::var("FLEDGE_AI_PROVIDER").ok())
        .or_else(|| config.ai.provider.clone())
}

/// Build a concrete provider from config + env + overrides. See
/// `resolve_provider_kind` for the precedence rules; model overrides follow
/// the same order.
pub fn build_provider(
    config: &Config,
    overrides: &ProviderOverride,
) -> Result<Box<dyn LlmProvider>> {
    let kind = resolve_provider_kind(config, overrides.provider.as_deref())?;

    // Emit the `claude` deprecation warning only when actually building a
    // provider (not during status/introspection) and only when the user
    // explicitly selected `claude`.
    if let Some(s) = selected_provider_string(config, overrides.provider.as_deref()) {
        if s.trim().eq_ignore_ascii_case("claude") {
            eprintln!(
                "warning: provider 'claude' is deprecated and now uses the Anthropic API directly. \
                 Set ai.provider = \"anthropic\" (and ANTHROPIC_API_KEY). The alias is removed in fledge 2.0."
            );
        }
    }

    let env_model = std::env::var("FLEDGE_AI_MODEL").ok();

    match kind {
        ProviderKind::Anthropic => {
            // Env wins, then new `ai.anthropic.*`, then deprecated `ai.claude.*`.
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| config.ai.anthropic.api_key.clone())
                .or_else(|| config.ai.claude.api_key.clone())
                .filter(|k| !k.is_empty());
            let model = overrides
                .model
                .clone()
                .or(env_model)
                .or_else(|| config.ai.anthropic.model.clone())
                .or_else(|| config.ai.claude.model.clone());
            build_corvid(
                "anthropic",
                model,
                api_key,
                config.ai.anthropic.base_url.clone(),
            )
        }
        ProviderKind::OpenAi => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .ok()
                .or_else(|| config.ai.openai.api_key.clone())
                .filter(|k| !k.is_empty());
            let model = overrides
                .model
                .clone()
                .or(env_model)
                .or_else(|| config.ai.openai.model.clone());
            build_corvid("openai", model, api_key, config.ai.openai.base_url.clone())
        }
        ProviderKind::Ollama => {
            let api_key = std::env::var("OLLAMA_API_KEY")
                .ok()
                .or_else(|| config.ai.ollama.api_key.clone())
                .filter(|k| !k.is_empty());
            let model = overrides
                .model
                .clone()
                .or(env_model)
                .unwrap_or_else(|| config.ai.ollama.model.clone());
            let host = resolve_effective_host(config, &model, &api_key);

            if is_cloud_model(&model) && api_key.is_none() {
                bail!(
                    "Cloud model '{}' requires authentication.\n  \
                     Set OLLAMA_API_KEY env var or run: fledge config set ai.ollama.api_key <key>",
                    model
                );
            }

            let timeout = resolve_ollama_timeout(config);
            Ok(Box::new(OllamaProvider {
                host,
                api_key,
                model,
                timeout,
            }))
        }
    }
}

/// Resolve a `corvid-ai` provider (anthropic / openai) from fledge's already
/// precedence-resolved key, model, and base URL.
fn build_corvid(
    name: &str,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<Box<dyn LlmProvider>> {
    let mut settings = corvid_ai::Settings::provider(name);
    settings.model = model;
    settings.api_key = api_key;
    settings.base_url = base_url;
    settings.timeout_secs = ai_timeout_override();

    let (provider, timeout) = corvid_ai::resolve(&settings).map_err(|e| anyhow::anyhow!("{e}"))?;
    let kind = match name {
        "openai" => ProviderKind::OpenAi,
        _ => ProviderKind::Anthropic,
    };
    Ok(Box::new(CorvidProvider {
        inner: provider,
        timeout,
        kind,
    }))
}

/// Human-friendly description of the active provider for pretty output.
pub fn describe(provider: &dyn LlmProvider) -> String {
    match provider.model_name() {
        Some(model) => format!("{} ({})", provider.kind().as_str(), model),
        None => provider.kind().as_str().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AiConfig, AnthropicConfig, OllamaConfig, OpenAiConfig};

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
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("FLEDGE_AI_TIMEOUT");
    }

    #[test]
    fn provider_kind_parses() {
        assert_eq!(
            ProviderKind::parse("anthropic").unwrap(),
            ProviderKind::Anthropic
        );
        assert_eq!(ProviderKind::parse("openai").unwrap(), ProviderKind::OpenAi);
        assert_eq!(ProviderKind::parse("ollama").unwrap(), ProviderKind::Ollama);
        // `claude` is a deprecated alias for `anthropic`.
        assert_eq!(
            ProviderKind::parse("claude").unwrap(),
            ProviderKind::Anthropic
        );
        assert_eq!(
            ProviderKind::parse("  Ollama ").unwrap(),
            ProviderKind::Ollama
        );
        assert!(ProviderKind::parse("nope").is_err());
    }

    #[test]
    fn resolve_defaults_to_anthropic() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        assert_eq!(
            resolve_provider_kind(&config, None).unwrap(),
            ProviderKind::Anthropic
        );
    }

    #[test]
    fn resolve_uses_config_provider() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ..Default::default()
            },
            ..Config::default()
        };
        assert_eq!(
            resolve_provider_kind(&config, None).unwrap(),
            ProviderKind::Ollama
        );
    }

    #[test]
    fn resolve_env_beats_config() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("FLEDGE_AI_PROVIDER", "ollama");
        let config = Config {
            ai: AiConfig {
                provider: Some("anthropic".into()),
                ..Default::default()
            },
            ..Config::default()
        };
        assert_eq!(
            resolve_provider_kind(&config, None).unwrap(),
            ProviderKind::Ollama
        );
        clear_env();
    }

    #[test]
    fn resolve_override_beats_env() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("FLEDGE_AI_PROVIDER", "ollama");
        let config = Config::default();
        assert_eq!(
            resolve_provider_kind(&config, Some("anthropic")).unwrap(),
            ProviderKind::Anthropic
        );
        clear_env();
    }

    #[test]
    fn claude_alias_resolves_to_anthropic() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        assert_eq!(
            resolve_provider_kind(&config, Some("claude")).unwrap(),
            ProviderKind::Anthropic
        );
    }

    #[test]
    fn build_anthropic_uses_key_and_model() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("anthropic".into()),
                anthropic: AnthropicConfig {
                    model: Some("claude-test".into()),
                    api_key: Some("sk-test".into()),
                    base_url: None,
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let provider = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(provider.kind(), ProviderKind::Anthropic);
        assert_eq!(provider.model_name(), Some("claude-test"));
    }

    #[test]
    fn build_anthropic_reads_deprecated_claude_config() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("anthropic".into()),
                claude: crate::config::ClaudeConfig {
                    model: Some("legacy-model".into()),
                    api_key: Some("sk-legacy".into()),
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let provider = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(provider.model_name(), Some("legacy-model"));
    }

    #[test]
    fn build_anthropic_without_key_errors() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("anthropic".into()),
                ..Default::default()
            },
            ..Config::default()
        };
        match build_provider(&config, &ProviderOverride::default()) {
            Err(e) => assert!(e.to_string().contains("API key"), "unexpected error: {e}"),
            Ok(_) => panic!("expected an error when no Anthropic key is set"),
        }
    }

    #[test]
    fn build_openai_requires_model() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("openai".into()),
                openai: OpenAiConfig {
                    api_key: Some("sk-test".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        // openai has no built-in default model.
        match build_provider(&config, &ProviderOverride::default()) {
            Err(e) => assert!(e.to_string().contains("model"), "unexpected error: {e}"),
            Ok(_) => panic!("expected an error when no OpenAI model is set"),
        }
    }

    #[test]
    fn build_openai_with_model_and_base_url() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("openai".into()),
                openai: OpenAiConfig {
                    api_key: Some("sk-test".into()),
                    model: Some("gpt-test".into()),
                    base_url: Some("https://openrouter.ai/api/v1".into()),
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let provider = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(provider.kind(), ProviderKind::OpenAi);
        assert_eq!(provider.model_name(), Some("gpt-test"));
    }

    #[test]
    fn build_ollama_respects_env_host_and_key() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("OLLAMA_HOST", "https://cloud.example.com");
        std::env::set_var("OLLAMA_API_KEY", "secret-token");
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ..Default::default()
            },
            ..Config::default()
        };
        let provider = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(provider.kind(), ProviderKind::Ollama);
        assert_eq!(provider.model_name(), Some("llama3.3"));
        clear_env();
    }

    #[test]
    fn build_ollama_model_precedence_override_env_config() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ollama: OllamaConfig {
                    model: "from-config".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };

        let p = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(p.model_name(), Some("from-config"));

        std::env::set_var("FLEDGE_AI_MODEL", "from-env");
        let p = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(p.model_name(), Some("from-env"));

        let p = build_provider(
            &config,
            &ProviderOverride {
                provider: None,
                model: Some("from-override".into()),
            },
        )
        .unwrap();
        assert_eq!(p.model_name(), Some("from-override"));
        clear_env();
    }

    #[test]
    fn ollama_generate_url_joins_cleanly() {
        let p = OllamaProvider {
            host: "http://localhost:11434".into(),
            api_key: None,
            model: "llama3.3".into(),
            timeout: Duration::from_secs(600),
        };
        assert_eq!(p.generate_url(), "http://localhost:11434/api/generate");

        let p = OllamaProvider {
            host: "https://cloud.example.com/".into(),
            api_key: None,
            model: "llama3.3".into(),
            timeout: Duration::from_secs(600),
        };
        assert_eq!(p.generate_url(), "https://cloud.example.com/api/generate");
    }

    #[test]
    fn describe_includes_model_when_set() {
        let p = OllamaProvider {
            host: "http://localhost:11434".into(),
            api_key: None,
            model: "llama3.3".into(),
            timeout: Duration::from_secs(600),
        };
        assert_eq!(describe(&p), "ollama (llama3.3)");
    }

    #[test]
    fn resolve_timeout_defaults_to_config() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    timeout_seconds: 42,
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        assert_eq!(resolve_ollama_timeout(&config), Duration::from_secs(42));
    }

    #[test]
    fn resolve_timeout_env_beats_config() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("FLEDGE_AI_TIMEOUT", "7");
        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    timeout_seconds: 42,
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        assert_eq!(resolve_ollama_timeout(&config), Duration::from_secs(7));
        clear_env();
    }

    #[test]
    fn is_cloud_model_matches_variants() {
        assert!(is_cloud_model("qwen3-coder:480b-cloud"));
        assert!(is_cloud_model("llama3-cloud"));
        assert!(is_cloud_model("model-cloud:latest"));
        assert!(!is_cloud_model("llama3.3"));
        assert!(!is_cloud_model("qwen3-coder:480b"));
        assert!(!is_cloud_model("cloudflare-model"));
    }

    #[test]
    fn resolve_host_local_by_default() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let host = resolve_effective_host(&config, "llama3.3", &None);
        assert_eq!(host, "http://localhost:11434");
    }

    #[test]
    fn resolve_host_routes_cloud_model_when_key_present() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let key = Some("test-key".to_string());
        let host = resolve_effective_host(&config, "qwen3-coder:480b-cloud", &key);
        assert_eq!(host, DEFAULT_OLLAMA_CLOUD_HOST);
    }

    #[test]
    fn resolve_host_env_var_wins_over_cloud_auto() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("OLLAMA_HOST", "https://override.example.com");
        let config = Config::default();
        let key = Some("test-key".to_string());
        let host = resolve_effective_host(&config, "qwen3-coder:480b-cloud", &key);
        assert_eq!(host, "https://override.example.com");
        clear_env();
    }

    #[test]
    fn build_ollama_cloud_model_without_key_errors() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ollama: OllamaConfig {
                    model: "qwen3-coder:480b-cloud".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        match build_provider(&config, &ProviderOverride::default()) {
            Err(e) => assert!(
                e.to_string().contains("requires authentication"),
                "unexpected error: {e}"
            ),
            Ok(_) => panic!("expected error for cloud model without API key"),
        }
    }

    #[test]
    fn empty_api_key_treated_as_none() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ollama: OllamaConfig {
                    api_key: Some("".into()),
                    model: "qwen3-coder:480b-cloud".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        match build_provider(&config, &ProviderOverride::default()) {
            Err(e) => assert!(
                e.to_string().contains("requires authentication"),
                "unexpected error: {e}"
            ),
            Ok(_) => panic!("expected error for cloud model with empty API key"),
        }
    }
}
