---
spec: prs.spec.md
---

## Test Plan

### Unit Tests

- Draft detection from `draft` field
- State display mapping (open, closed, merged)
- Icon selection logic (filled vs open circle)
- Diff stat formatting (files changed, additions, deletions)

### Integration Tests

- `fledge prs` lists PRs from a real repo (requires network and token)
- `fledge prs view <number>` displays expected fields including diff stats
- `fledge prs --json` returns valid JSON array
