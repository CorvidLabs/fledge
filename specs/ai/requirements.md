---
spec: ai.spec.md
---

## User Stories

- As a user new to fledge's LLM integration, I want a single command (`fledge ai use`) that walks me through picking a provider and model without reading docs
- As an agent or shell script, I want `fledge ai use <provider> <model>` to persist the choice in one non-interactive call
- As a user switching between local Ollama and Ollama Cloud, I want `fledge ai models` to show me exactly what my daemon reports via `/api/tags` — not a stale hardcoded list
- As a user debugging "why did `fledge ask` pick this model?", I want `fledge ai status` to tell me not just what's active but *where* the value came from (env / config / default)

## Acceptance Criteria

### REQ-ai-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge ai status` matches the provider/model/host that `llm::build_provider` would resolve; regression between the two is a tracked bug
### REQ-ai-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge ai models --provider ollama --json` parses cleanly with `jq` and includes at least `name` per model
### REQ-ai-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge ai use <provider> <model>` writes `ai.provider` + per-provider `model` to `~/.config/fledge/config.toml` atomically
### REQ-ai-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge ai use` in `--non-interactive` without a provider arg errors via `utils::require_interactive`
### REQ-ai-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge ai use ollama` interactively, with a running daemon, offers a Select with the live model list
### REQ-ai-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Unknown providers on any `--provider` flag reject at clap parse time (not at runtime)

## Constraints

- Must use the existing `ureq` / `dialoguer` / `serde_json` deps — no new crates
- Must not duplicate provider-resolution logic that lives in `llm::build_provider`; status calls the same underlying pieces
- Interactive pickers must support the `(custom…)` escape hatch so users can enter a model that isn't in the live list yet

## Out of Scope

- Pulling models (`ollama pull`) from within fledge — users still run that via the Ollama CLI
- Claude model catalog live-discovery — Anthropic's SDK doesn't expose a `/models` endpoint the claude CLI surfaces, and maintaining a hardcoded authoritative list drifts faster than it helps; the curated list is guidance only
- Per-command (ephemeral) provider switch via `fledge ai use --once` — users already have `--provider` / `--model` flags on `ask` and `review` for that
