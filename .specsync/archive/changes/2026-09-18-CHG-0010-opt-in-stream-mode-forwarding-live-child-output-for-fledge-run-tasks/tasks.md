---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: tasks
---

# Tasks

- [x] `--stream` flag on `fledge run`, threaded through cli/main/watch
- [x] `pump` generic tee helper
- [x] `run_streaming` + `StreamedOutput`, stdin inherited
- [x] `--json` path routed through streaming when `--stream` is set
- [x] Buffered capture unchanged as the default
- [x] Unit tests: pump byte-equality, partial line, empty input, >8 KiB payload
- [x] Unit tests: stream separation, exit-code propagation, parity with `Command::output`
- [x] Integration tests: default stays buffered, envelope still filled under streaming,
      failing-task envelopes equal across modes, deps stream, human-mode no-op
- [x] `specs/run/` bumped 6 -> 7 with invariants and requirements
- [x] CLI reference documents the intended use for long-running/interactive commands
- [x] fmt, clippy and tests green
