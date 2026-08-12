---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: design
---

# Design

## Surface

A single boolean flag, `fledge run --stream`. `RunOptions` gains `stream: bool`.
Substantive logic lives in `src/run.rs`; `src/cli.rs`, `src/main.rs` and
`src/watch.rs` only declare and forward the flag.

## Mirror target: stderr, not stdout

Under `--stream --json` both pipes are teed — bytes are written to fledge's **stderr**
as they arrive and simultaneously accumulated into buffers that populate the envelope's
`stdout` and `stderr` fields.

Mirroring to stdout was rejected: it would interleave child bytes with the JSON envelope
and break every `| jq` consumer. Rejecting the `--stream --json` combination outright
was also rejected, because `--json` is the only mode that actually buffers, so refusing
it would deny the feature to the case that motivated the issue.

The envelope's field set and `schema_version` are unchanged; `stdout`/`stderr` are
never silently emptied by streaming.

## Ordering guarantee

Each stream is forwarded in order, and `write_all` on `io::Stderr` locks per chunk, so
a chunk is never split by the other stream. Relative interleaving **between** stdout and
stderr is best-effort: two OS pipes drained by two threads cannot reconstruct the child's
true write order. The only design that preserves cross-stream order — a single shared fd —
would make `stdout` and `stderr` inseparable in the envelope, which is a worse trade.
True interleaving remains available on the inherited-terminal path.

The issue's "forwarded in order" is therefore met per-stream, not cross-stream, and that
limit is stated rather than glossed.

## Non-TTY behavior

No TTY probe: `--stream` forwards unconditionally. A flag whose effect silently vanishes
in CI is both surprising and untestable, and live logs are precisely what a long CI task
wants. Output is forwarded verbatim — no colour, prefixes or line framing — so a pipe
receives exactly the child's bytes.

## Implementation

`pump<R: Read, W: Write>` performs the tee and is generic so it unit-tests against
in-memory buffers instead of racing real pipes. `run_streaming` returns
`StreamedOutput { status, stdout, stderr }`. It writes through `io::stderr()` rather
than holding a `StderrLock` for the stream's lifetime, which would starve the other
thread until its pipe closed and defeat the purpose.

`--stream` propagates to dependency tasks, since it is an output mode rather than a task
input. `--stream` without `--json` is an accepted, documented no-op so wrappers can
pass it unconditionally.
