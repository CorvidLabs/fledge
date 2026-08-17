---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: context
---

# Context

Issue #507 asked for live child output during `fledge run`, on the grounds that stdout
and stderr are invisible until a task exits.

Investigation showed that premise is true for only one of the two paths. The
human-readable path calls `Command::status()`, which **inherits** fledge's stdio, so
output there was always live and correctly interleaved. The `--json` path calls
`Command::output()`, which buffers both pipes until exit and closes the child's stdin.

So the real defect is narrower and sharper than the issue states: `--json` is the mode
that goes dark, and it is also the mode an agent or CI wrapper is most likely to use for
a long-running task. Recording the correction here so the scope is not mistaken later for
"fledge run was entirely buffered", which it was not.

A second consequence of `Command::output()` is that the child's stdin is closed, so an
interactive task under `--json` cannot prompt. Streaming restores stdin inheritance,
which is a real behavioral difference and is captured as an invariant rather than left
implicit.
