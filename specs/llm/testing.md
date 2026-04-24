---
spec: llm.spec.md
---

## Test Plan

### Unit Tests

In `src/llm.rs`:

- `provider_kind_parses` — case-insensitive, trims whitespace, rejects unknown names
- `resolve_defaults_to_claude` — no config, no env, no override → `Claude`
- `resolve_uses_config_provider` — `ai.provider = "ollama"` is respected
- `resolve_env_beats_config` — `FLEDGE_AI_PROVIDER` overrides config
- `resolve_override_beats_env` — CLI `--provider` overrides env
- `build_ollama_respects_env_host_and_key` — `OLLAMA_HOST` + `OLLAMA_API_KEY` populate the built provider
- `build_claude_respects_model_override` — `--model` flag surfaces in the ClaudeProvider
- `build_ollama_model_precedence_override_env_config` — override > env > config > default
- `ollama_generate_url_joins_cleanly` — trailing slash tolerated, `/api/generate` path joined
- `describe_includes_model_when_set` — pretty formatter includes model
- `describe_bare_when_no_model` — `"claude"` alone when model is None
- `resolve_timeout_defaults_to_config` — no env var → use `ai.ollama.timeout_seconds`
- `resolve_timeout_env_beats_config` — `FLEDGE_AI_TIMEOUT` wins over config
- `resolve_timeout_ignores_bad_env` — non-integer env value falls through to config
- `build_ollama_applies_timeout_from_config` — `build_provider` populates `OllamaProvider.timeout`

All tests that mutate env vars serialize on a static Mutex to avoid parallel-test races.

### Integration Tests

- `fledge ask --help` advertises `--provider` and `--model`
- `fledge review --help` advertises `--provider`
- `fledge config set ai.provider ollama` and related keys parse and round-trip

Not tested in CI (requires live endpoints):

- End-to-end `fledge ask "..."` against a running Ollama daemon — manual, run locally
- Ollama Cloud / Turbo auth — manual, run with a real key
- Response decoding edge cases (truncated streams, timeouts) — manual

### Manual Test Recipe (for the author's Ollama Pro test)

```bash
# Local Ollama
ollama serve &
ollama pull llama3.3
export FLEDGE_AI_PROVIDER=ollama
fledge ask "how does the work module build branch names?"
fledge ask --with-specs work "why does it sanitize names this way?"
fledge review --format checklist

# Ollama Cloud / Turbo
export OLLAMA_HOST=https://<cloud-host>
export OLLAMA_API_KEY=<your-key>
fledge ask --model <cloud-model> "give me an architecture summary"

# Per-invocation override
unset FLEDGE_AI_PROVIDER
fledge ask --provider ollama --model llama3.3:70b "local, verbose run"
fledge ask --provider claude "compare to claude's answer"
```

### Regression Watch

- If a future change adds a new provider, confirm `resolve_provider_kind` and `build_provider` pattern-match on every `ProviderKind` variant (the compiler enforces this)
- If Ollama changes its response schema, `OllamaGenerateResponse` will fail to deserialize and the invoke will return a clear error — add a test for the new shape before bumping
