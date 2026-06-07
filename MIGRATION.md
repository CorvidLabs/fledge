# Migration notes

## 1.5.0 — AI providers move to plain HTTP (no `claude` CLI)

fledge's AI commands (`ask`, `review`, `ai`) no longer shell out to the `claude`
CLI. They now call provider APIs directly over HTTP through the
[`corvid-ai`](https://crates.io/crates/corvid-ai) crate. Three providers ship in
core:

- **`anthropic`** (the new default) — Anthropic Messages API.
- **`openai`** — any OpenAI-compatible Chat Completions endpoint (OpenAI,
  OpenRouter, Groq, DeepSeek, Mistral, xAI, Together, local servers), selected by
  `ai.openai.base_url`.
- **`ollama`** — unchanged.

### What you need to do

| If you were using... | Do this |
|----------------------|---------|
| `ai.provider = "claude"` (or the default) | Set `ai.provider = "anthropic"` and provide a key: `ANTHROPIC_API_KEY` env var or `fledge config set ai.anthropic.api_key <key>`. The `claude` alias still works (with a deprecation warning) and routes to the Anthropic API. |
| `ai.claude.model` / `ai.claude.api_key` | Still read as a fallback (deprecated). Prefer `ai.anthropic.model` / `ai.anthropic.api_key`. |
| Ollama | Nothing changes. |
| The `claude` CLI for auth | No longer used. fledge needs an API key, not the CLI. |

### Deprecations (removed in 2.0)

- `ai.provider = "claude"` — use `anthropic`.
- `ai.claude.model` / `ai.claude.api_key` — use `ai.anthropic.*`.

### New config keys

```
ai.provider          # anthropic | openai | ollama
ai.anthropic.model
ai.anthropic.api_key      # or ANTHROPIC_API_KEY
ai.anthropic.base_url     # optional override
ai.openai.base_url        # gateway, e.g. https://openrouter.ai/api/v1
ai.openai.api_key         # or OPENAI_API_KEY
ai.openai.model           # required (no default)
```

### New capability

Any OpenAI-compatible gateway now works out of the box:

```bash
fledge config set ai.provider openai
fledge config set ai.openai.base_url https://openrouter.ai/api/v1
fledge config set ai.openai.api_key sk-or-...
fledge config set ai.openai.model anthropic/claude-sonnet-4-6
fledge ask "explain this module"
```
