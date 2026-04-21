---
spec: checks.spec.md
---

## Test Plan

### Unit Tests

- `format_duration` correctly formats seconds into human-readable strings (e.g., 12 → "12s", 90 → "1m 30s")
- `current_branch` returns an error with guidance when in detached HEAD state

### Integration Tests

- `fledge checks` in a git repo with a GitHub remote returns a formatted check list or "No CI checks found"
- `fledge checks --json` outputs valid JSON
- `fledge checks --branch nonexistent` handles missing branches gracefully
