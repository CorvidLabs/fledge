---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: testing
---

# Testing

## REQ-run-020: opt-in live forwarding on the `--json` path

- Integration (`tests/run.rs`): with `--stream --json`, child bytes appear on fledge's
  stderr; without `--stream`, no child bytes appear there — this is what distinguishes the
  two modes without racing them.
- Integration: `--stream` without `--json` exits 0 and still prints the `Running task:`
  summary.
- Integration: dependency tasks stream as well.
- Manual: a 3s task under `--stream --json` showed `tick-1..3` on stderr at t=1.5s with
  stdout still empty, confirming liveness rather than a flush at exit.

## REQ-run-021: the envelope is not weakened by streaming

- Unit (`src/run.rs`): `run_streaming_matches_buffered_capture` asserts byte-parity with
  `Command::output` for the same command.
- Integration: `--stream --json` mirrors both streams **and** still fills the envelope's
  `stdout`/`stderr`.
- Integration: a failing task reports `exit_code: 3` and the streamed and buffered
  envelopes are asserted **equal**.
- Integration: stdout remains a single parseable JSON document even when the task itself
  prints JSON-shaped text.

## REQ-run-022: honest ordering, and working prompts

- Unit: `pump` byte-equality for capture and mirror; a partial line (a prompt with no
  trailing newline) is forwarded; empty input; a payload larger than the 8 KiB buffer.
- Unit: `run_streaming` keeps the two streams separate and propagates a non-zero exit
  (`exit 7`), retaining output written before the failure.
- Manual: an interactive `read` task produced `hello-bruno` under `--stream` versus
  `hello-` (empty, stdin closed) on the buffered path.
- Code review: cross-stream interleaving is explicitly not asserted anywhere, because it is
  not guaranteed; only per-stream order is tested.

## Determinism

No test depends on a timing race. Liveness is asserted by the presence or absence of child
bytes on fledge's stderr, and `pump` is generic over `Read`/`Write` so it is exercised
against in-memory buffers rather than real pipes.

## Rejection signals

- Child bytes appearing on **stdout** under `--stream --json` — this would corrupt the
  envelope for every `| jq` consumer.
- The envelope's `stdout`/`stderr` arriving empty, or the field set or `schema_version`
  changing, when `--stream` is passed.
- Any difference in `exit_code` between the streamed and buffered paths.
- Child output appearing when `--stream` was not passed — the default must stay buffered.
