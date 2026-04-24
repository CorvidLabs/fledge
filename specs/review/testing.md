---
spec: review.spec.md
---

## Test Plan

### Unit Tests

- Default branch detection (main vs master)
- Empty diff detection
- Diff stat line parsing
- File filter flag applied to git diff command
- `build_prompt` includes the diff
- `build_prompt` format variants (summary, checklist, inline)
- `build_prompt` respects custom focus prompt
- `build_prompt` includes spec context when provided, omits it when None
- `build_prompt` spec-context block contains the review-scope constraint language (`CRITICAL`, `context only`, "Do NOT suggest changes", "Do NOT critique or review the specs")
- `build_spec_context` returns None when `--no-auto-specs` and no `--with-specs`
- `build_spec_context` merges auto-detected and explicit modules, deduped and sorted
- `build_spec_context` honors `--no-auto-specs` (skips auto-detection but still loads explicit bundles if any)

### Integration Tests

- `fledge review` runs against a branch with changes (requires Claude CLI)
- `fledge review --file <path>` restricts diff to one file
- Empty diff produces a clear error message
- `fledge review --help` advertises `--with-specs` and `--no-auto-specs`

### Not tested (by design)

- End-to-end LLM output quality — depends on Claude's behavior, not fledge. Manual verification by authors.
- No test that Claude obeys the "review only the diff" constraint in practice — we can only enforce the *instruction* in the prompt
