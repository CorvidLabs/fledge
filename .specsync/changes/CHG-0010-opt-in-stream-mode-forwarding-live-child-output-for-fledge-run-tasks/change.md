---
id: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
state: accepted
type: feature
base_commit: 8ce6fc4e94d25bcc78d37cb029daa46e338993af
---

# Opt-in --stream mode forwarding live child output for fledge run tasks

## Intent

Opt-in --stream mode forwarding live child output for fledge run tasks

## Affected Canonical Specs

- `run`

## Acceptance Criteria

- fledge run --stream --json forwards child stdout and stderr to fledge's stderr as they arrive while still filling the envelope's stdout and stderr fields exactly as the buffered path did; the envelope field set and schema_version are unchanged; buffered capture remains the default when --stream is absent; exit codes are propagated identically in both modes; streamed and buffered envelopes for the same failing task are equal; --stream without --json is an accepted no-op that preserves the task summary; cargo test, cargo clippy --all-targets -- -D warnings and cargo fmt --check are green.

## No-spec Rationale

Not applicable
