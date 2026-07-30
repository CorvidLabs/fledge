---
spec: doctor.spec.md
---

## Test Plan

### Unit Tests

- Version string extraction from command output (handles `v` and `go` prefixes, trailing punctuation)
- AI section reports active provider correctly under config / env-var / default precedence
- Section JSON serialization includes `informational: bool`
- Pass/fail totals exclude informational sections
- `probe_ollama_host_true_when_tags_endpoint_answers` — `/api/tags` probe against `test_support::MockHttpServer`, with and without a trailing slash on the host
- `probe_ollama_host_false_when_unreachable_or_erroring` — a closed loopback port and a 5xx both report unreachable

### Integration Tests

Every `tests/doctor.rs` case runs inside `common::TempEnv`: isolated `HOME`/`FLEDGE_CONFIG_DIR`, provider API keys stripped, and `OLLAMA_HOST` pointed at a closed loopback port — so the AI probe can never reach a real endpoint and the developer's config is never read or written (issue #447).

- `fledge doctor` runs without panic in a valid project (`tests/doctor.rs`)
- `fledge doctor --json` outputs valid JSON with all four sections
- An unreachable AI host is reported in the `AI` section without failing the command
- `doctor` writes nothing into the config directory
- Missing toolchain entries render dimmed in text output and as `status: "missing"` in JSON
- Failing non-informational checks include actionable fix suggestions
- Probe timeout fires when a binary hangs longer than 10 seconds
