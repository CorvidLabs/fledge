---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: requirements
---

# Requirements

## REQ-run-020: opt-in live forwarding on the `--json` path

As someone running a long or interactive task through `fledge run --json`, I want child
output as it happens, so I can see progress, prompts and diagnostics instead of waiting
for the task to exit.

- `--stream` is opt-in; omitting it leaves capture buffered.
- The flag applies to the `--json` path, which is the one that buffers. The
  human-readable path already inherits fledge's stdio and was never dark.
- `--stream` without `--json` is accepted and documented as a no-op so wrappers may pass
  it unconditionally.
- Dependency tasks stream too, since `--stream` is an output mode rather than a task input.

## REQ-run-021: the envelope is not weakened by streaming

As a tool parsing `run --json`, I need the envelope to be exactly what it was, so adding
`--stream` upstream cannot break me.

- `stdout` and `stderr` are populated identically to the buffered path.
- The field set and `schema_version` are unchanged.
- Streamed and buffered envelopes are equal for the same task, including `exit_code`.
- Child bytes go to stderr, never stdout, so stdout stays a single JSON document.

## REQ-run-022: honest ordering, and working prompts

As someone reading streamed output, I need to know what ordering I can rely on, and I need
interactive tasks to actually work.

- Each stream is forwarded in order; a chunk is never split by the other stream.
- Cross-stream interleaving is best-effort and documented as a limitation, because two OS
  pipes drained by two threads cannot reconstruct the child's true write order.
- stdin is inherited under `--stream`, so prompts work — the buffered path closes stdin.
- Output is verbatim: no colour, prefixes or line framing.
