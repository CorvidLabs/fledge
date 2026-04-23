---
spec: lanes.spec.md
---

## Test Plan

### Unit Tests

- Lane definition parsing (sequential, parallel, inline steps)
- Task reference validation (unknown task detection)
- Step type deserialization
- fail_fast flag defaults to true

### Integration Tests

- `fledge lanes` lists available lanes
- `fledge lanes run ci` executes steps in order
- `fledge lanes run ci --dry-run` prints plan without executing
- Parallel steps run concurrently
- Missing task reference fails before execution
- `fledge lanes init` adds default lanes to fledge.toml
