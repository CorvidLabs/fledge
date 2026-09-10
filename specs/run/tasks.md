---
spec: run.spec.md
---

## Tasks

- [x] Write run spec
- [x] Implement RunOptions struct with task, init, and list fields
- [x] Implement detect_project_type() for Rust, Node, Go, Python, Ruby, Java, and generic
- [x] Implement fledge.toml parsing with short-form and full-form task support
- [x] Implement task execution with shell dispatch
- [x] Implement task dependency resolution with execution ordering
- [x] Implement circular dependency detection
- [x] Implement --init to scaffold a starter fledge.toml from detected project type
- [x] Implement --list to display available tasks
- [x] Implement auto-detection fallback when no fledge.toml exists
- [x] Wire RunAction subcommand into main.rs
- [x] Add unit tests for detect_project_type, task parsing, circular dependency detection
- [x] Register spec and verify with cargo test, clippy, fmt, spec-check
- [x] Add `--stream` flag to the run subcommand and `RunOptions` (#507)
- [x] Implement `pump` (tee: forward a chunk, capture it) and `run_streaming` (two piped streams, two threads, inherited stdin)
- [x] Wire `--stream` into the `--json` execution path, mirroring to stderr so the envelope stays pure
- [x] Propagate `--stream` to dependency tasks
- [x] Unit-test `pump` (capture/mirror equality, partial lines, empty input, multi-buffer payloads) and `run_streaming` (stream separation, exit code, parity with buffered capture)
- [x] Integration-test mirroring, buffered default, envelope purity, exit-code parity, deps, and `--stream` without `--json`
- [x] Degrade gracefully when mirroring fails: keep capturing, stop echoing, warn once, still emit the envelope with the real exit code (#509 review)
- [x] Join both forwarding threads before propagating either one's failure, so neither is left detached and writing (#509 review)
- [x] Select shell syntax per platform in the streaming integration tests (`;` for `sh -c`, `&` for `cmd /C`) so Windows CI exercises them too (#509 review)

## Gaps

- No parallel task execution
- No task caching/skip-if-unchanged
- `--stream --json` guarantees per-stream ordering only; cross-stream interleaving is best-effort
- Lanes steps have no equivalent streaming mode
- `--json` emits one envelope per executed task, so a task with `deps` produces a JSON *stream* rather than a single document. Pre-existing, documented rather than changed — collapsing it into one envelope would be a breaking change to the `run_task` contract and belongs in its own change
