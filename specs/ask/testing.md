---
spec: ask.spec.md
---

## Test Plan

### Unit Tests

- Argument joining (multiple words become one question string)
- Empty question detection
- Claude CLI availability check

### Integration Tests

- `fledge ask <question>` returns a response (requires Claude CLI)
- `fledge ask` with no arguments shows usage hint
