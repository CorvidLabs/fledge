---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
module: run
---

## ADDED

### REQUIREMENT REQ-run-020

`fledge run` SHALL accept an opt-in `--stream` flag that forwards child stdout and
stderr live on the `--json` execution path, which otherwise buffers both pipes until the
task exits.

Acceptance Criteria
- With `--stream --json`, child bytes appear on fledge's stderr while the task is still running.
- Without `--stream`, capture remains buffered — the default behavior is unchanged.
- `--stream` without `--json` is accepted as a documented no-op and preserves the task summary.
- `--stream` propagates to dependency tasks.

### REQUIREMENT REQ-run-021

Streaming SHALL NOT weaken the `--json` envelope.

Acceptance Criteria
- The envelope's `stdout` and `stderr` fields are populated identically to the buffered path.
- The envelope field set and `schema_version` are unchanged.
- For the same failing task, the streamed and buffered envelopes are equal, including `exit_code`.
- Child bytes are mirrored to stderr, never to stdout, so stdout remains a single JSON document.

### REQUIREMENT REQ-run-022

Streaming SHALL forward each stream in order and inherit stdin.

Acceptance Criteria
- Each of stdout and stderr is forwarded in order, and a chunk is never split by the other stream.
- Cross-stream interleaving is best-effort and documented as such; it is not guaranteed.
- The child inherits stdin under `--stream`, so an interactive task can prompt — unlike the
  buffered path, which closes stdin.
- Output is forwarded verbatim, with no colour, prefixes or line framing added.
