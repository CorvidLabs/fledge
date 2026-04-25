---
spec: doctor.spec.md
---

## Test Plan

### Unit Tests

- Version string extraction from command output (handles `v` and `go` prefixes, trailing punctuation)
- AI section reports active provider correctly under config / env-var / default precedence
- Section JSON serialization includes `informational: bool`
- Pass/fail totals exclude informational sections

### Integration Tests

- `fledge doctor` runs without panic in a valid project (`tests/doctor.rs`)
- `fledge doctor --json` outputs valid JSON with all four sections
- Missing toolchain entries render dimmed in text output and as `status: "missing"` in JSON
- Failing non-informational checks include actionable fix suggestions
- Probe timeout fires when a binary hangs longer than 10 seconds
