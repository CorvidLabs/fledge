---
spec: ask.spec.md
---

## Test Plan

### Unit Tests

- Argument joining (multiple words become one question string)
- Empty question detection
- Claude CLI availability check
- `build_prompt` includes the question
- `build_prompt` adds the JSON instruction only when `json=true`
- `build_prompt` includes the spec context block when provided, omits it when `None`
- `expand_with_specs` flattens comma-separated and repeated flags, dedups, handles whitespace
- `expand_with_specs("all", …)` returns every module name sorted

### Integration Tests

- `fledge ask <question>` returns a response (requires Claude CLI)
- `fledge ask` with no arguments shows usage hint
- `fledge ask --help` advertises `--with-specs` and `--no-spec-index`

### Not tested (by design)

- End-to-end LLM quality — this depends on Claude's behavior, not fledge
- Network-dependent tests are deliberately omitted from the CI suite
