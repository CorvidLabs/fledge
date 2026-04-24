---
spec: ai.spec.md
---

## Context

### Why this exists

The v0.13 LLM provider work (see `specs/llm/`) landed `--provider` / `--model` flags plus env + config keys, but selecting and verifying an active provider still required either editing `config.toml` by hand or typing long `FLEDGE_AI_PROVIDER=… FLEDGE_AI_MODEL=…` prefixes on every call. The `llm/tasks.md` spec flagged this as the planned follow-up. `fledge ai` closes that gap with three ergonomic subcommands that delegate to the same `Config` / `llm::ProviderKind` primitives — no parallel state, no drift risk.

### Where the logic lives

- Provider resolution (what's active) is duplicated on purpose: `llm::build_provider` resolves for *invocation*, `ai::resolve_provider_with_source` resolves for *reporting* with source tags. Both read the same env vars and config keys. The `regression watch` note in `testing.md` calls out the invariant: these two must agree.
- Live model listing for Ollama calls `/api/tags` — the same endpoint `fledge doctor` already probes for reachability, so there's prior art for timeouts and error surfacing.
- Claude has no live catalog endpoint exposed through the CLI, so `CLAUDE_WELL_KNOWN_MODELS` is a curated hint. The UI labels it as non-authoritative.

### Trade-offs taken

- **No subcommand bloat**: `status`, `models`, `use` are the minimum viable set. `ai pull`, `ai login`, `ai benchmark` were considered and punted — each can grow out of this skeleton without reshaping it.
- **Interactive by default with a non-interactive escape hatch**: positional args (`fledge ai use ollama qwen3-coder:480b-cloud`) let agents skip the TTY entirely. This matches the pattern `fledge work start` uses.
- **Source tags in `ai status`**: costlier to maintain than a bare "current value" print, but explains *why* a config edit didn't take effect ("Model: llama3.3 (from env)" → "oh, I still have `FLEDGE_AI_MODEL` set in my shell"). Worth the extra code.
- **`(custom…)` escape in the model picker**: the live list is sometimes missing a model the user just wants to try (e.g. before `ollama pull`). Free-text fallback keeps the picker from being a dead-end.

### Alternatives considered and rejected

- **Put all three subcommands under `fledge config ai …`**: rejected — they're read/live/interactive flows, not config-CRUD. `fledge config` stays for scalar key/value edits.
- **Make `fledge ai` shell out to the `ollama` CLI for model listing**: rejected — `/api/tags` is stable, deps-free (ureq already in the tree), and works against remote Cloud endpoints where the local `ollama` CLI can't see remote state.
- **Store "last used model" history**: rejected as scope creep for v1. `ai status` is a pure report of current state; add history in a follow-up only if users ask for it.
