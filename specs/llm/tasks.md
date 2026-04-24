---
spec: llm.spec.md
---

## Tasks

- [x] Design `LlmProvider` trait with `invoke` / `kind` / `model_name`
- [x] Implement `ClaudeProvider` wrapping current `claude --print` behavior
- [x] Implement `OllamaProvider` — HTTP POST to `/api/generate` via existing ureq
- [x] Optional Bearer auth on Ollama requests (for Cloud / Turbo / self-hosted auth)
- [x] `resolve_provider_kind` and `build_provider` with correct precedence (override > env > config > default)
- [x] Extend `Config` with `ai.provider`, `ai.claude.model`, `ai.ollama.{host,api_key,model}`
- [x] Refactor `ask.rs` and `review.rs` to go through the trait — prompt composition untouched
- [x] Add `--provider` flag to `ask` and `review`; `ask` also gains `--model`
- [x] Update `fledge doctor` to detect both providers, show the active one, and probe reachability of the Ollama endpoint via `/api/tags`
- [x] Unit tests: provider kind parsing, precedence resolution, Ollama URL joining, model precedence, describe formatting

## Gaps

- Live model-catalog UX landed — see `specs/ai/` (`fledge ai status` / `models` / `use`)
- `ClaudeProvider` still shells out to the `claude` CLI; a direct Anthropic API impl would remove the external-process dependency but adds key management surface we don't need yet
- Ollama streaming not implemented; large prompts block until the model finishes
- No retry on transient HTTP failures — a single connection error fails the invocation

## Review Sign-offs

- **Product**: in progress (pending end-to-end testing against user's Ollama Pro)
- **QA**: done (11 unit tests)
- **Design**: n/a
- **Dev**: done
