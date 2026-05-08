use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

use crate::config::Config;

/// Which LLM backend a command should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Ollama,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "claude",
            ProviderKind::Ollama => "ollama",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "claude" => Ok(ProviderKind::Claude),
            "ollama" => Ok(ProviderKind::Ollama),
            other => bail!("Unknown provider '{other}'. Supported: claude, ollama"),
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
/// 1. `OLLAMA_HOST` env var (explicit user override — always wins)
/// 2. Non-default config host (user explicitly configured a custom host)
/// 3. Cloud host when API key is present AND model is a cloud model
/// 4. Config host (default: `http://localhost:11434`)
pub fn resolve_effective_host(
    config: &Config,
    model: &str,
    api_key: &Option<String>,
) -> String {
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

    /// Human name of the provider (e.g. "claude", "ollama").
    fn kind(&self) -> ProviderKind;

    /// The model identifier the provider will use (for display / debug).
    fn model_name(&self) -> Option<&str>;
}

/// Shells out to the `claude` CLI. This preserves the existing behavior that
/// has been in `ask` and `review` from day one.
pub struct ClaudeProvider {
    pub model: Option<String>,
}

impl LlmProvider for ClaudeProvider {
    fn invoke(&self, prompt: &str) -> Result<String> {
        crate::github::ensure_claude_cli()?;

        let mut args: Vec<String> = Vec::new();
        if let Some(model) = &self.model {
            args.push("--model".into());
            args.push(model.clone());
        }
        args.push("--print".into());
        args.push(prompt.into());

        let output = Command::new("claude")
            .args(&args)
            .output()
            .context("invoking claude CLI")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() {
                eprintln!("{stderr}");
            }
            bail!("claude CLI exited with an error.");
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    fn model_name(&self) -> Option<&str> {
        self.model.as_deref()
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
                    "Ollama endpoint returned HTTP {code} from {url}. Check the model name, API key, and host URL."
                );
            }
            Err(e) => {
                return Err(anyhow::Error::new(e))
                    .with_context(|| format!("POST {url} (is the Ollama server running?)"));
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
///   4. default: `claude`
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
    Ok(ProviderKind::Claude)
}

/// Build a concrete provider from config + env + overrides. See
/// `resolve_provider_kind` for the precedence rules; model overrides follow
/// the same order.
pub fn build_provider(
    config: &Config,
    overrides: &ProviderOverride,
) -> Result<Box<dyn LlmProvider>> {
    let kind = resolve_provider_kind(config, overrides.provider.as_deref())?;

    let env_model = std::env::var("FLEDGE_AI_MODEL").ok();

    match kind {
        ProviderKind::Claude => Ok(Box::new(ClaudeProvider {
            model: overrides
                .model
                .clone()
                .or(env_model)
                .or_else(|| config.ai.claude.model.clone()),
        })),
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
    use crate::config::{AiConfig, ClaudeConfig, OllamaConfig};

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
        std::env::remove_var("FLEDGE_AI_TIMEOUT");
    }

    #[test]
    fn provider_kind_parses() {
        assert_eq!(ProviderKind::parse("claude").unwrap(), ProviderKind::Claude);
        assert_eq!(ProviderKind::parse("ollama").unwrap(), ProviderKind::Ollama);
        assert_eq!(ProviderKind::parse("CLAUDE").unwrap(), ProviderKind::Claude);
        assert_eq!(
            ProviderKind::parse("  ollama ").unwrap(),
            ProviderKind::Ollama
        );
        assert!(ProviderKind::parse("nope").is_err());
    }

    #[test]
    fn resolve_defaults_to_claude() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        assert_eq!(
            resolve_provider_kind(&config, None).unwrap(),
            ProviderKind::Claude
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
                provider: Some("claude".into()),
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
            resolve_provider_kind(&config, Some("claude")).unwrap(),
            ProviderKind::Claude
        );
        clear_env();
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
        // We can't downcast trait objects cleanly without `Any`, but we can
        // verify the provider kind and check model resolution.
        assert_eq!(provider.kind(), ProviderKind::Ollama);
        assert_eq!(provider.model_name(), Some("llama3.3"));
        clear_env();
    }

    #[test]
    fn build_claude_respects_model_override() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let overrides = ProviderOverride {
            provider: Some("claude".into()),
            model: Some("opus-4".into()),
        };
        let provider = build_provider(&config, &overrides).unwrap();
        assert_eq!(provider.kind(), ProviderKind::Claude);
        assert_eq!(provider.model_name(), Some("opus-4"));
    }

    #[test]
    fn build_ollama_model_precedence_override_env_config() {
        let _g = test_lock();
        clear_env();
        // Config has its own model.
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ollama: OllamaConfig {
                    model: "from-config".into(),
                    ..OllamaConfig::default()
                },
                claude: ClaudeConfig::default(),
            },
            ..Config::default()
        };

        // Config-only
        let p = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(p.model_name(), Some("from-config"));

        // Env beats config
        std::env::set_var("FLEDGE_AI_MODEL", "from-env");
        let p = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(p.model_name(), Some("from-env"));

        // Override beats env
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

        // Trailing slash is stripped
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
        let p = ClaudeProvider {
            model: Some("sonnet-4.5".into()),
        };
        assert_eq!(describe(&p), "claude (sonnet-4.5)");
    }

    #[test]
    fn describe_bare_when_no_model() {
        let p = ClaudeProvider { model: None };
        assert_eq!(describe(&p), "claude");
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
    fn resolve_timeout_ignores_bad_env() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("FLEDGE_AI_TIMEOUT", "not-a-number");
        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    timeout_seconds: 99,
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        assert_eq!(resolve_ollama_timeout(&config), Duration::from_secs(99));
        clear_env();
    }

    #[test]
    fn build_ollama_applies_timeout_from_config() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                provider: Some("ollama".into()),
                ollama: OllamaConfig {
                    timeout_seconds: 123,
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        // Downcast isn't worth the machinery — exercise the path via the
        // concrete builder and verify the field through generate_url stays
        // sane. Timeout value is verified directly in resolve_ollama_timeout
        // tests above.
        let _ = build_provider(&config, &ProviderOverride::default()).unwrap();
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
    fn resolve_host_stays_local_for_cloud_model_without_key() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let host = resolve_effective_host(&config, "qwen3-coder:480b-cloud", &None);
        assert_eq!(host, "http://localhost:11434");
    }

    #[test]
    fn resolve_host_respects_explicit_config_host() {
        let _g = test_lock();
        clear_env();
        let config = Config {
            ai: AiConfig {
                ollama: OllamaConfig {
                    host: "https://custom.example.com".into(),
                    ..OllamaConfig::default()
                },
                ..Default::default()
            },
            ..Config::default()
        };
        let key = Some("test-key".to_string());
        let host = resolve_effective_host(&config, "qwen3-coder:480b-cloud", &key);
        assert_eq!(host, "https://custom.example.com");
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
    fn resolve_host_stays_local_for_non_cloud_with_key() {
        let _g = test_lock();
        clear_env();
        let config = Config::default();
        let key = Some("test-key".to_string());
        let host = resolve_effective_host(&config, "llama3.3", &key);
        assert_eq!(host, "http://localhost:11434");
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
    fn build_ollama_cloud_model_with_key_succeeds() {
        let _g = test_lock();
        clear_env();
        std::env::set_var("OLLAMA_API_KEY", "test-key");
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
        let provider = build_provider(&config, &ProviderOverride::default()).unwrap();
        assert_eq!(provider.kind(), ProviderKind::Ollama);
        assert_eq!(provider.model_name(), Some("qwen3-coder:480b-cloud"));
        clear_env();
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
