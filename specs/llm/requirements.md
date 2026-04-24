---
spec: llm.spec.md
---

## User Stories

- As a user who prefers local models (privacy, offline, cost), I want `fledge ask` and `fledge review` to work against my local Ollama daemon with no code changes beyond a config or env-var switch
- As a user of Ollama Cloud / Turbo, I want to point fledge at the cloud endpoint and pass my API key, without giving up spec-aware prompting
- As an agent, I want to choose my provider per-invocation via `--provider` flag so I can mix local (cheap, fast) and cloud (stronger) per task
- As a maintainer, I want the prompt composition in `ask` / `review` to remain provider-agnostic so adding a new provider is localized to one file

## Acceptance Criteria

- Default behavior (no config, no env, no flag) is identical to the pre-v0.13 Claude-CLI-only behavior
- Setting `ai.provider = "ollama"` in config OR `FLEDGE_AI_PROVIDER=ollama` in env routes all AI commands through Ollama
- Per-invocation `--provider ollama` overrides both env and config
- Model selection follows the same override > env > config > default precedence
- Ollama's HTTP request shape matches the `/api/generate` endpoint's published schema
- `fledge doctor` reports both providers and which is active
- No regression: `fledge ask --json` and `fledge review --json` outputs remain parseable; payloads gain a `provider` and `model` field

## Constraints

- Must use the existing `ureq` dependency — no new HTTP client
- Must not break existing ReviewOptions / AskOptions call sites outside main.rs (CLI glue) — structs gain fields with serde defaults or `Default` impls
- Must not leak the Ollama API key in debug output, logs, or error messages
- All provider impls are `Send + Sync` so they can live behind `Box<dyn LlmProvider>`

## Out of Scope

- Streaming response bodies (Ollama supports `stream: true`; current impl uses `false`). Can be added later without breaking the trait
- OpenAI-compatible endpoints beyond Ollama — would be a third provider, deliberately deferred until someone wants it
- Automatic model discovery (live-listing installed Ollama models) — see `fledge ai models` in the next PR
- Telemetry (token counts, cost) — not instrumenting until we know what users want tracked
