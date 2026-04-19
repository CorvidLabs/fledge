---
spec: issues.spec.md
---

## Test Plan

### Unit Tests

- PR filtering logic removes items with `pull_request` key
- State parameter mapping (open, closed, all)
- Label query parameter encoding
- PR detection in view mode (item has `pull_request` field)

### Integration Tests

- `fledge issues` lists issues from a real repo (requires network and token)
- `fledge issues view <number>` displays expected fields
- `fledge issues --json` returns valid JSON array
