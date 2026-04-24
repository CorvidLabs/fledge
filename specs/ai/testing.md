---
spec: ai.spec.md
---

## Test Plan

### Unit Tests

In `src/ai.rs`:

- `status_provider_defaults_report_default_source` — no env, no config → `Claude` + `Source::Default`
- `status_provider_env_source` — `FLEDGE_AI_PROVIDER=ollama` → `Source::Env`
- `status_provider_config_source` — `ai.provider = "ollama"` in config → `Source::ConfigFile`
- `ollama_host_config_vs_default` — bare default vs user-set host, source tagging correct
- `ollama_host_env_wins` — `OLLAMA_HOST` env overrides config
- `ollama_model_env_wins_over_config` — `FLEDGE_AI_MODEL` env beats config
- `claude_model_absent_when_unset` — claude model is `None` with no env and no config

Tests that mutate env serialize on a static Mutex to avoid parallel-test races.

### Integration Tests

In `tests/integration.rs`:

- `cli_ai_help_lists_subcommands` — `status`, `models`, `use` all appear in `fledge ai --help`
- `cli_ai_status_json_shape` — `fledge ai status --json` parses, contains `provider` + `provider_source`
- `cli_ai_use_rejects_unknown_provider_at_parse_time` — clap `value_parser` rejects `ai use gpt`
- `cli_ai_models_rejects_unknown_provider_at_parse_time` — clap rejects `ai models --provider gemini`
- `cli_ai_use_non_interactive_without_provider_fails` — `--non-interactive ai use` with no args errors with a clear hint

### Not Tested in CI (Requires Live Endpoints)

- `fledge ai models --provider ollama` against a running daemon — manual; verified by author during v0.13 dogfooding
- Interactive `fledge ai use` TTY flow — manual; dialoguer doesn't expose a hook for scripted input in this codebase

### Regression Watch

- If `llm::build_provider` changes its precedence order, `src/ai.rs` resolvers (`resolve_provider_with_source`, `resolve_ollama_model`, `resolve_ollama_host`) must change in lockstep or `fledge ai status` will lie to the user
- If Ollama changes `/api/tags` response shape, `OllamaTagsResponse` deserialization fails loudly — good — but a new field we want to surface (e.g. a human-readable size) needs an additive `ModelEntry` field to preserve JSON schema stability
