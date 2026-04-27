---
module: llm
version: 3
status: active
files:
  - src/llm.rs

db_tables: []
depends_on:
  - config
  - github
---

# Llm

## Purpose

Provider abstraction for LLM-backed commands. `fledge ask` and `fledge review` both delegate to an `LlmProvider` implementation, letting users choose between the Claude CLI (default) and any Ollama-compatible endpoint (local daemon, Ollama Cloud/Turbo, self-hosted mirrors) without the command code knowing which backend is active. Spec-aware prompt composition is provider-agnostic — the same prompt text flows to whichever backend the user selected.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `LlmProvider` | Trait all providers implement |
| `ProviderKind` | Enum: `Claude`, `Ollama` |
| `as_str` | `ProviderKind` method — returns `"claude"` or `"ollama"` for display / JSON output |
| `parse` | `ProviderKind` method — case-insensitive parser; trims whitespace; errors on unknown values |
| `ClaudeProvider` | Wraps the existing `claude` CLI shell-out |
| `OllamaProvider` | POSTs `{model, prompt, stream:false}` to `<host>/api/generate`, optional Bearer auth |
| `ProviderOverride` | `{ provider: Option<String>, model: Option<String> }` — per-invocation overrides |
| `resolve_provider_kind` | Determine active provider given config + override |
| `build_provider` | Construct the concrete provider box from config + env + overrides |
| `normalize_ollama_host` | Ensures `OllamaProvider.host` always has a scheme; prepends `http://` to bare `host:port` values |
| `describe` | Human string: `"claude (sonnet-4.5)"` or `"ollama (llama3.3)"` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ProviderKind` | `Claude` or `Ollama` |
| `ClaudeProvider` | `{ model: Option<String> }` |
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
| `resolve_provider_kind` | `(&Config, Option<&str>) -> Result<ProviderKind>` | CLI override > `FLEDGE_AI_PROVIDER` env > `ai.provider` config > `Claude` |
| `build_provider` | `(&Config, &ProviderOverride) -> Result<Box<dyn LlmProvider>>` | Builds a concrete provider; model follows the same precedence order |
| `normalize_ollama_host` | `(&str) -> String` | Trims whitespace and trailing `/`; prepends `http://` when no scheme is present |
| `describe` | `(&dyn LlmProvider) -> String` | Pretty formatter for spinner messages and JSON payloads |
| `ProviderKind::parse` | `(&str) -> Result<Self>` | Case-insensitive parse; trims whitespace |
| `ProviderKind::as_str` | `(&self) -> &'static str` | `"claude"` or `"ollama"` |

## Invariants

1. Precedence for active provider (highest to lowest): explicit CLI override > `FLEDGE_AI_PROVIDER` env var > `ai.provider` in config > default `"claude"`
2. Precedence for active model follows the same order: CLI `--model` > `FLEDGE_AI_MODEL` env > per-provider config field > provider default
3. `OllamaProvider.host` defaults to `http://localhost:11434`; `normalize_ollama_host` trims whitespace, strips trailing slashes, and prepends `http://` when no scheme is present (so bare `localhost:11434` becomes `http://localhost:11434`)
4. When `OllamaProvider.api_key` is set (via `OLLAMA_API_KEY` env or `ai.ollama.api_key` config), the request sends `Authorization: Bearer <key>`; otherwise no auth header is sent
5. `ClaudeProvider` preserves the exact behavior `ask` / `review` had before this module existed: shells out to `claude --print <prompt>` with optional `--model <name>`
6. `OllamaProvider.invoke` POSTs `{"model": ..., "prompt": ..., "stream": false}` to `<host>/api/generate` and parses `{"response": "..."}` from the reply
7. Network errors from Ollama surface with the full URL in the message so users can diagnose host / port / daemon-down issues quickly
8. No provider implementation modifies the prompt text — spec context composition stays in `ask` / `review`
9. `OllamaProvider.timeout` is resolved by `build_provider` with precedence `FLEDGE_AI_TIMEOUT` env var (seconds, integer) > `ai.ollama.timeout_seconds` config > default 600s; a non-integer env value is ignored and falls through to config

## Behavioral Examples

### Default provider is Claude (unchanged from pre-v0.13)
```
$ fledge ask "what does the trust module do?"
● Thinking (claude):
[Claude's answer]
```

### Switch to Ollama via env var
```
$ export FLEDGE_AI_PROVIDER=ollama
$ fledge ask "what does the trust module do?"
● Thinking (ollama (llama3.3)):
[Local-model answer]
```

### Switch to Ollama Cloud / Turbo
```
$ fledge config set ai.provider ollama
$ fledge config set ai.ollama.host https://ollama.com
$ fledge config set ai.ollama.api_key sk-...
$ fledge config set ai.ollama.model "claude-sonnet-4.5"  # if the endpoint supports it
$ fledge ask --with-specs work "why does work sanitize branch names?"
```

### Per-invocation override
```
$ fledge ask --provider ollama --model llama3.3:70b "quick question"
$ fledge review --provider claude --model opus-4
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Unknown provider string | CLI/env/config contains something other than `claude` or `ollama` | Bail with "Unknown provider 'X'. Supported: claude, ollama" |
| `claude` CLI not installed | Active provider is `claude` | Bail via existing `github::ensure_claude_cli` check |
| Ollama host unreachable | POST to `<host>/api/generate` fails | Bail with the full URL and a "(is the Ollama server running?)" hint |
| Malformed Ollama response | `/api/generate` returns non-JSON or missing `response` field | Bail with decoding error |

## Dependencies

- `config` module — reads `ai.*` section
- `github` module — `ensure_claude_cli` check for the Claude path
- `ureq` — HTTP to Ollama endpoints (already a fledge dep)
- `serde_json` — request encoding, response decoding

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-04-27 | Document `normalize_ollama_host` — ensures bare `host:port` values get an `http://` scheme; update invariant 3 to describe full normalization behavior |
| 2 | 2026-04-24 | `OllamaProvider` gains a `timeout: Duration` field populated by `build_provider`; adds `ai.ollama.timeout_seconds` config fallback so the per-request timeout is tunable without env vars (`FLEDGE_AI_TIMEOUT` still wins) |
| 1 | 2026-04-23 | Initial spec — provider abstraction with Claude + Ollama implementations, env-var and config resolution, CLI overrides |
