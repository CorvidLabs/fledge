---
spec: ai.spec.md
---

## Tasks

- [x] `fledge ai status` — reports active provider/model/host with source tags (env / config / default)
- [x] `fledge ai status --json` — machine-readable `StatusReport`
- [x] `fledge ai models` — live `/api/tags` query for Ollama, curated list for Claude
- [x] `fledge ai models --search <q>` — case-insensitive substring filter
- [x] `fledge ai models --json` — structured output for agents
- [x] `fledge ai use <provider> [<model>]` — non-interactive positional form
- [x] `fledge ai use` — interactive picker; Ollama selects from live `/api/tags`
- [x] `--non-interactive` gate on `ai use` when args are missing
- [x] Unit tests for source resolution (env > config > default) mirroring `llm::build_provider`
- [x] Integration tests for `--help`, rejected providers, `--json` shape, `--non-interactive` error path

## Gaps

- No `fledge ai pull <model>` — intentionally deferred; users run `ollama pull` directly
- Claude model list is curated and will drift as Anthropic ships new aliases; acceptable for now since it's flagged as non-authoritative in the UI
- No "last-used" history — `ai status` shows current state only, not what was previously active

## Review Sign-offs

- **Product**: done
- **QA**: done (unit + integration)
- **Design**: n/a
- **Dev**: done
