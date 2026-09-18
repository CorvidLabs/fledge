---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: plan
---

# Plan

1. Add the `--stream` flag to the `Run` variant in `src/cli.rs`; thread it through
   `src/main.rs` and `src/watch.rs` into `RunOptions`.
2. Implement `pump` as a generic tee over `Read`/`Write`.
3. Implement `run_streaming` returning `StreamedOutput`, spawning a reader per pipe and
   inheriting stdin.
4. Route the `--json` execution path through `run_streaming` when `--stream` is set,
   leaving `Command::output()` in place otherwise.
5. Keep the human path on `Command::status()` unchanged.
6. Unit-test `pump` and `run_streaming`, including parity with `Command::output`.
7. Integration-test both modes, exit codes, dependency propagation and envelope equality.
8. Update `specs/run/` (6 -> 7) and the CLI reference docs.
