---
spec: llm.spec.md
---

## Context

Before v0.13, `src/ask.rs` and `src/review.rs` directly shelled out to the `claude` CLI. That locked fledge into one provider and made private, offline, and cloud-alternative workflows impossible without editing code. The `llm` module abstracts the "send prompt, get answer" operation so the command code can stay identical while the user picks which backend runs.

The abstraction point is deliberately small — one trait method, two impls — so adding future providers is contained.

## Related Modules

- `config` — extended with `[ai]` section for provider selection and per-provider settings
- `ask` / `review` — the two consumers; both refactored to invoke the trait instead of `claude` directly
- `doctor` — detects both providers, reports which is active and whether reachable
- `github` — existing `ensure_claude_cli` check is still used by `ClaudeProvider`

## Design Decisions

- **Two providers in core, not plugins.** Claude was already in core (via the `claude` CLI shell-out); adding Ollama as a plugin while Claude stays in core would have been asymmetric. A plugin-contributed provider capability can be added later when someone wants a non-core provider — until then, the plugin protocol's IPC ceremony is unjustified for "string in, string out."
- **`OllamaProvider` is a single impl for every Ollama-speaking endpoint.** Local daemon, Cloud / Turbo, self-hosted, OpenAI-style compatibility mirrors — all look the same from the client's perspective (host + optional API key + model). Modeling each as a separate provider would bloat the abstraction.
- **HTTP, not `ollama run`.** Spec-aware prompts can be tens of kilobytes. Piping via argv hits platform limits and causes escape-handling bugs; stdin piping works but duplicates plumbing. The HTTP API is canonical, handles large payloads cleanly, and is ~30 LOC with the existing `ureq` dependency.
- **Precedence rule: override > env > config > default.** Mirrors the GitHub-token resolution already in `config.rs`. CLI flags are the highest-priority lever; env vars let agent shells set and forget; config is persistent; default preserves the pre-v0.13 behavior for users who do nothing.
- **Prompt composition stays in `ask` / `review`.** The LLM module knows nothing about specs, diffs, or review formats — it just sends a string and returns a string. That keeps the provider surface minimal and testable in isolation.
- **`ProviderKind::Claude` default** keeps existing users on the exact same code path they had before.

## Files to Read First

- `src/llm.rs` — complete module (~250 LOC + tests)
- `src/config.rs` lines around `AiConfig` / `ClaudeConfig` / `OllamaConfig` — config schema
- `src/ask.rs` and `src/review.rs` — see how the trait plugs in
- `src/doctor.rs` `check_ai` — reachability probe and active-provider display

## 1.0 Provider Contract

For the 1.0 release, the two in-tree provider implementations are pinned to the following transports:

- **`ProviderKind::Claude`** — shells out to the `claude` CLI. Authentication, model selection, and rate-limiting are delegated entirely to that binary. Requires the `claude` CLI on `PATH` (verified by `doctor`).
- **`ProviderKind::Ollama`** — HTTP against an Ollama-compatible endpoint (local daemon, Cloud / Turbo, self-hosted, OpenAI-compatible mirrors). Uses `/api/generate` for single-shot prompts.

These are the *guaranteed* transports for `provider = "claude"` and `provider = "ollama"`. Adding a new transport for an existing kind (e.g. an Anthropic-API-direct path for Claude) would be a breaking change to user expectations and is out of scope for v1. Adding a *new* `ProviderKind` variant (e.g. `ProviderKind::Anthropic`, `ProviderKind::OpenAI`) is purely additive and may happen at any point in the 1.x line.

This contract is what plugin authors, agents, and dotfile config can rely on — no silent transport swaps under an existing provider name.

## Open Questions

- Should Ollama's `/api/chat` (multi-turn) be used instead of `/api/generate` (single-prompt)? Our prompts are already single-shot, so `/api/generate` is the correct match. Keep.
