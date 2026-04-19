---
spec: work.spec.md
---

## Test Plan

### Unit Tests

- Branch name sanitization (spaces, special chars, uppercase)
- Default branch detection
- Commit count parsing
- PR URL extraction from gh output

### Integration Tests

- `fledge work start` creates a branch in a temp git repo
- `fledge work status` shows branch info
