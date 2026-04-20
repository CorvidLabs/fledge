---
spec: doctor.spec.md
---

## Test Plan

### Unit Tests

- Version string extraction from command output
- Project type to toolchain mapping
- Check status formatting

### Integration Tests

- `fledge doctor` runs without panic in a valid project
- `fledge doctor --json` outputs valid JSON
- Missing tool produces actionable fix suggestion
