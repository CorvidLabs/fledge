---
spec: flows.spec.md
---

## Test Plan

### Unit Tests

- Flow definition parsing (sequential, parallel, inline steps)
- Task reference validation (unknown task detection)
- Step type deserialization
- fail_fast flag defaults to true

### Integration Tests

- `fledge flow` lists available flows
- `fledge flow ci` executes steps in order
- `fledge flow ci --dry-run` prints plan without executing
- Parallel steps run concurrently
- Missing task reference fails before execution
- `fledge flow --init` adds default flows to fledge.toml
