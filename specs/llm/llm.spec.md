---
module: llm
version: 6
status: active
files:
  - src/llm.rs

db_tables: []
depends_on:
  - config
---

# Llm

## Purpose

Provider abstraction for LLM-backed commands. `fledge ask` and `fledge review` delegate to an `LlmProvider` implementation, letting users pick a backend without the command code knowing which is active. As of 1.5.0 everything is plain HTTP: `anthropic` (default) and `openai` (any OpenAI-compatible endpoint) are served by the [`corvid-ai`](https://crates.io/crates/corvid-ai) crate, and `ollama` keeps fledge's native client for local/cloud routing and `/api/tags`. There is no CLI shell-out. Spec-aware prompt composition is provider-agnostic; the same prompt text flows to whichever backend is selected.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `LlmProvider` | Trait all providers implement |
| `ProviderKind` | Enum: `Anthropic`, `OpenAi`, `Ollama` |
| `as_str` | `ProviderKind` method — returns `"anthropic"`, `"openai"`, or `"ollama"` |
| `parse` | `ProviderKind` method — case-insensitive; trims; `claude` is a deprecated alias of `anthropic`; errors on unknown values |
| `CorvidProvider` | Wraps a `corvid-ai` provider (Anthropic native or any OpenAI-compatible endpoint) |
| `OllamaProvider` | POSTs `{model, prompt, stream:false}` to `<host>/api/generate`, optional Bearer auth |
| `ProviderOverride` | `{ provider: Option<String>, model: Option<String> }` — per-invocation overrides |
| `resolve_provider_kind` | Determine active provider given config + override |
| `build_provider` | Construct the concrete provider box from config + env + overrides |
| `normalize_ollama_host` | Ensures `OllamaProvider.host` always has a scheme |
| `resolve_effective_host` | Picks the Ollama host: env var > custom config > cloud auto-route > default localhost |
| `is_cloud_model` | Returns `true` when a model tag contains `-cloud` |
| `DEFAULT_OLLAMA_CLOUD_HOST` | `"https://ollama.com"` |
| `describe` | Human string: `"anthropic (claude-sonnet-4-6)"` or `"ollama (llama3.3)"` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ProviderKind` | `Anthropic`, `OpenAi`, or `Ollama` |
| `CorvidProvider` | `{ inner: corvid_ai::Provider, timeout: Duration, kind: ProviderKind }` |
| `OllamaProvider` | `{ host, api_key: Option<String>, model, timeout: Duration }` |
| `ProviderOverride` | Per-invocation overrides (CLI flags bypass env and config) |
| `OllamaGenerateResponse` | (private) The `{ response: String }` payload decoded from `/api/generate` |

### Traits

| Trait | Description |
|-------|-------------|
| `LlmProvider` | `fn invoke(&self, prompt: &str) -> Result<String>; fn kind(&self) -> ProviderKind; fn model_name(&self) -> Option<&str>;` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `resolve_provider_kind` | `(&Config, Option<&str>) -> Result<ProviderKind>` | CLI override > `FLEDGE_AI_PROVIDER` env > `ai.provider` config > `Anthropic` |
| `build_provider` | `(&Config, &ProviderOverride) -> Result<Box<dyn LlmProvider>>` | Builds a concrete provider; model follows the same precedence order |
| `normalize_ollama_host` | `(&str) -> String` | Trims whitespace and trailing `/`; prepends `http://` when no scheme |
| `describe` | `(&dyn LlmProvider) -> String` | Pretty formatter for spinner messages and JSON payloads |
| `resolve_effective_host` | `(&Config, &str, &Option<String>) -> String` | Cloud-aware Ollama host resolution |
| `is_cloud_model` | `(&str) -> bool` | True when model name contains `-cloud` |
| `ProviderKind::parse` | `(&str) -> Result<Self>` | Case-insensitive parse; `claude` aliases to `anthropic` |
| `ProviderKind::as_str` | `(&self) -> &'static str` | `"anthropic"`, `"openai"`, or `"ollama"` |

## Invariants

1. Provider precedence (highest to lowest): explicit CLI override > `FLEDGE_AI_PROVIDER` env > `ai.provider` config > default `"anthropic"`.
2. Model precedence follows the same order: CLI `--model` > `FLEDGE_AI_MODEL` env > per-provider config field > provider default.
3. `claude` is accepted everywhere `anthropic` is, as a deprecated alias. `build_provider` prints a one-line deprecation warning to stderr only when the user explicitly selected `claude` (not during status/introspection). Removed in fledge 2.0.
4. `anthropic` is served by `corvid-ai`'s Anthropic Messages provider; it requires an API key (`ANTHROPIC_API_KEY` env > `ai.anthropic.api_key` > deprecated `ai.claude.api_key`). Model falls back `ai.anthropic.model` > deprecated `ai.claude.model` > crate default.
5. `openai` is served by `corvid-ai`'s OpenAI-compatible provider against `ai.openai.base_url` (default OpenAI). A model id is required (no built-in default). Key is `OPENAI_API_KEY` env > `ai.openai.api_key`; a missing key for a keyless/local endpoint is allowed by the crate.
6. `OllamaProvider` behavior is unchanged: `normalize_ollama_host` trims and adds a scheme; Bearer auth when a key is set; POSTs `{"model", "prompt", "stream": false}` to `<host>/api/generate` and parses `{"response"}`.
7. Network errors from Ollama surface the full URL plus an `OLLAMA_HOST` hint when that env var is set.
8. No provider implementation modifies the prompt text — spec context composition stays in `ask` / `review`.
9. Ollama timeout precedence: `FLEDGE_AI_TIMEOUT` env (integer seconds) > `ai.ollama.timeout_seconds` > 600s. The corvid-backed providers use `FLEDGE_AI_TIMEOUT` when set, else the crate default.
10. Ollama cloud auto-routing (`-cloud` model + key → `https://ollama.com`) is unchanged; explicit `OLLAMA_HOST` or non-default config host take priority.
11. `build_provider` bails when an Ollama cloud model is selected with no key. The corvid-backed Anthropic path bails when no key is configured.
12. Empty API key strings (config or env) are treated as absent.

## Behavioral Examples

### Default provider is Anthropic (API key required)
```
$ export ANTHROPIC_API_KEY=sk-ant-...
$ fledge ask "what does the trust module do?"
● Thinking [anthropic (claude-sonnet-4-6)]:
[answer]
```

### Any OpenAI-compatible gateway
```
$ fledge config set ai.provider openai
$ fledge config set ai.openai.base_url https://openrouter.ai/api/v1
$ fledge config set ai.openai.api_key sk-or-...
$ fledge config set ai.openai.model anthropic/claude-sonnet-4-6
$ fledge ask "why does work sanitize branch names?"
```

### Ollama (unchanged), and a deprecated alias
```
$ fledge ask --provider ollama --model llama3.3:70b "quick question"
$ fledge review --provider claude --model claude-opus-4-8   # warns, routes to anthropic
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Unknown provider string | Value other than `anthropic`/`openai`/`ollama`/`claude` | Bail "Unknown provider 'X'. Supported: anthropic, openai, ollama" |
| Missing Anthropic key | Active provider `anthropic` and no key in env/config | Bail with API-key setup guidance (from `corvid-ai`) |
| Missing OpenAI model | Active provider `openai` and no model set | Bail "missing model" (no default) |
| Ollama host unreachable | POST to `<host>/api/generate` fails | Bail with the full URL and a "(is the Ollama server running?)" hint |
| Cloud model without API key | `-cloud` model but no `OLLAMA_API_KEY` / `ai.ollama.api_key` | Bail with "requires authentication" |

## Dependencies

- `config` module — reads `ai.*` section
- `corvid-ai` — Anthropic + OpenAI-compatible HTTP (sync leaf crate)
- `ureq` — HTTP to Ollama endpoints
- `serde_json` — Ollama request encoding / response decoding

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 6 | 2026-06-07 | Drop the `claude` CLI shell-out. `anthropic` (new default) and `openai` (any OpenAI-compatible endpoint) are served over HTTP by the `corvid-ai` crate via `CorvidProvider`; `ProviderKind` becomes `Anthropic`/`OpenAi`/`Ollama` with `claude` a deprecated alias of `anthropic`. New `ai.anthropic.*` / `ai.openai.*` config (deprecated `ai.claude.*` still read). `ollama` and the `github`/`ensure_claude_cli` dependency are removed from this module |
| 5 | 2026-05-11 | `ClaudeProvider` gains `api_key`, forwarded to the `claude` CLI; Ollama errors append an `OLLAMA_HOST` hint |
| 4 | 2026-05-08 | Add cloud auto-routing: `resolve_effective_host`, `is_cloud_model`, `DEFAULT_OLLAMA_CLOUD_HOST` |
| 3 | 2026-04-27 | Document `normalize_ollama_host` scheme prepend |
| 2 | 2026-04-24 | `OllamaProvider` gains a `timeout` field; `ai.ollama.timeout_seconds` fallback |
| 1 | 2026-04-23 | Initial spec, provider abstraction with Claude + Ollama implementations |
