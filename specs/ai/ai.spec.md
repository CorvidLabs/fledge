---
module: ai
version: 1
status: active
files:
  - src/ai.rs

db_tables: []
depends_on:
  - config
  - llm
---

# Ai

## Purpose

`fledge ai` is the ergonomic surface for picking an AI provider and model without editing `config.toml` or typing long env exports. Three subcommands:

- `fledge ai status` — what's active right now, and where each value came from (env / config / default).
- `fledge ai models` — live list of models available for a provider (Ollama: `/api/tags`; Claude: a short curated alias list).
- `fledge ai use [provider] [model]` — interactive picker with a live model list for Ollama; fully scriptable via positional args.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — dispatches to the three subcommand handlers |
| `AiAction` | Enum with `Status`, `Models`, `Use` variants from CLI parsing |

### Structs & Enums

| Type | Description |
|------|-------------|
| `AiAction` | `Status { json }`, `Models { provider, search, json }`, `Use { provider, model }` |
| `Source` | Private — `Env` / `ConfigFile` / `Default` tag on resolved values for status reporting |
| `StatusReport` | Serializable — `provider`, `provider_source`, `model`, `model_source`, `host`, `host_source` |
| `ModelsReport` | Serializable — `provider` + `models: Vec<ModelEntry>` |
| `ModelEntry` | Serializable — `name`, optional `family` / `parameter_size` / `quantization` / `size_bytes` / `remote_host` |
| `OllamaTagsResponse` | Private — decodes `{ models: [...] }` from Ollama's `/api/tags` |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(AiAction) -> Result<()>` | Top-level dispatch |

## Invariants

1. `fledge ai status` reports the same provider/model/host that `build_provider` in `llm` would pick — precedence parity with `fledge ask` / `fledge review` is required; drift between the two is a bug
2. `Source` distinguishes `Env`, `ConfigFile`, and `Default` so the user can see *why* a value is active; the label is rendered in the human output as `(from env)` / `(from config)` / `(from default)`
3. `fledge ai models --provider ollama` queries `<host>/api/tags` live with a 5-second timeout; a failure surfaces the full URL plus a "(is the Ollama server running?)" hint, not a silent empty list
4. `fledge ai models --provider claude` returns a small, intentionally curated alias list (not authoritative — claude CLI accepts arbitrary aliases); the human output prints a dim trailing note to that effect
5. `--search <q>` is a case-insensitive substring filter on `name`; applied after fetching, so the remote call is unchanged
6. `--json` output is stable-shaped: `status` emits the full `StatusReport`, `models` emits `{provider, models: [...]}`; never an array at the top level
7. `fledge ai use <provider> <model>` is fully non-interactive and writes to `~/.config/fledge/config.toml` — agents can script it without TTY detection
8. `fledge ai use` with missing args enters an interactive picker; when stdin is not a TTY or `--non-interactive` is set, it errors via `utils::require_interactive("provider")` rather than hanging
9. Interactive Ollama model picker queries the live `/api/tags` list and offers a `(custom…)` entry so the user can still pick a not-yet-pulled model; on endpoint failure it falls back to a free-text `Input`
10. `fledge ai use` only writes the keys it resolves — if the user picks claude without a model, `ai.claude.model` is untouched (respecting "use claude's default")

## Behavioral Examples

### Status — active provider and where each value came from
```
$ fledge ai status
  Provider: ollama (from config)
     Model: qwen3-coder:480b-cloud (from config)
      Host: http://localhost:11434 (from default)

$ FLEDGE_AI_MODEL=codellama:latest fledge ai status
  Provider: ollama (from config)
     Model: codellama:latest (from env)
      Host: http://localhost:11434 (from default)
```

### Models — live Ollama list with filter + JSON
```
$ fledge ai models --provider ollama --search coder
  2 models for ollama:
    qwen3-coder:480b-cloud  [480B, qwen3moe, BF16, cloud → https://ollama.com:443]
    qwen3-coder-next:cloud  [80B, qwen3next, FP8, cloud → https://ollama.com:443]

$ fledge ai models --provider ollama --search coder --json | jq '.models[].name'
"qwen3-coder:480b-cloud"
"qwen3-coder-next:cloud"
```

### Use — non-interactive and interactive
```
$ fledge ai use ollama qwen3-coder:480b-cloud
✅ Active provider: ollama (qwen3-coder:480b-cloud)

$ fledge ai use                           # interactive picker
? Select AI provider › ollama
? Select Ollama model  › qwen3-coder:480b-cloud
✅ Active provider: ollama (qwen3-coder:480b-cloud)
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Unknown provider on CLI | `--provider gpt` or `ai use gpt` | clap `value_parser` rejects at parse time with a "possible values" hint |
| Ollama endpoint unreachable on `ai models` | daemon down or wrong host | Bail with the full URL and "(is the Ollama server running?)" |
| Ollama endpoint returns HTTP error on `ai models` | 401/404/5xx | Bail with "HTTP {code} from {url}. Check the host and API key." |
| `ai use` without args in `--non-interactive` | no TTY or flag set | `require_interactive("provider")` errors with a clear fix hint |
| `ai use ollama` with model omitted, endpoint unreachable | interactive fallback | falls back to free-text `Input` so the user can still pick a model |

## Dependencies

- `config` — reads `ai.*` keys; `ai use` mutates them via `Config::set` + `Config::save`
- `llm` — `ProviderKind::parse` / `ProviderKind::as_str` for consistent provider naming
- `utils` — `is_interactive` / `require_interactive` for TTY gating
- `dialoguer` — `Select`, `Input`, `ColorfulTheme` for interactive pickers
- `ureq` — HTTP GET to Ollama's `/api/tags`
- `serde_json` — JSON decode of `/api/tags` and `--json` output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-24 | Initial spec — `fledge ai status` / `models` / `use`. Status reports the *source* of each resolved value so users can tell env from config from default. `ai use` is interactive by default with a live Ollama model picker and a non-interactive positional form (`fledge ai use <provider> [<model>]`) for agents. |
