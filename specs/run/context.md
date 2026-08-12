---
spec: run.spec.md
---

## Key Decisions

- Tasks are defined in `fledge.toml` under `[tasks]` — either short form (`build = "cargo build"`) or full form with deps, env, dir, and description
- Task dependencies form a DAG — circular deps are detected before execution and produce a clear error
- `detect_project_type()` is public because it's used by `init` (to generate default `fledge.toml`) and `doctor` (for toolchain checks)
- `task_defaults()` returns sensible default tasks per language (build, test, lint, fmt) so new projects work out of the box
- `--list` shows available tasks with descriptions — useful for discoverability
- `--init` generates a starter `fledge.toml` based on detected project type
- `--stream` (#507) exists because the two execution paths differ in visibility. The human-readable path calls `Command::status()`, which inherits fledge's stdio — output was always live there. The `--json` path calls `Command::output()`, which buffers both streams until exit *and* closes the child's stdin, so a long-running or interactive task under `--json` shows nothing and cannot prompt. `--stream` closes that gap
- `--stream --json` mirrors to **stderr**, not stdout. Forwarding child stdout to fledge's stdout would interleave arbitrary child bytes with the JSON envelope and break every `| jq` consumer. Rejecting the flag combination was the alternative; mirroring to stderr was chosen because it delivers the feature (live progress, working prompts) while leaving the machine contract byte-for-byte intact — stdout is still exactly one envelope, and `stdout`/`stderr` in it are still complete and separated
- `--stream` does not probe for a TTY, even though the issue framed the feature as "when attached to a terminal". An explicit flag whose effect silently disappears in CI is untestable and surprising; live logs are also exactly what a CI run of a long task wants. Output is forwarded verbatim (no colour, no prefixes), so a pipe receives precisely the child's bytes
- Ordering is per-stream, not cross-stream, and the spec says so rather than over-claiming. Two pipes drained by two threads cannot reconstruct the child's true write order. The one design that *would* preserve it — handing the child a single fd for both streams — makes `stdout`/`stderr` inseparable in the envelope, which is a worse trade. `write_all` on `io::Stderr` takes the lock for one chunk, so chunks are at least never split mid-write
- `--stream` propagates to dependency tasks (unlike pass-through args, which are scoped to the named task): it describes how output is presented, not what a task does

## Files to Read First

- `src/run.rs` — task parsing, dependency resolution, execution
- `fledge.toml` (in any project) — the task definition format
- `specs/run/run.spec.md` — formal API and invariants

## Current Status

- Fully implemented: task parsing, dep resolution, execution with env/dir support
- Auto-detection covers: rust, node, go, python, ruby, java-gradle, java-maven, swift, generic
- `task_defaults()` provides starter tasks for each detected project type
- `--stream` implemented for the `--json` path via `run_streaming`/`pump`; a documented no-op elsewhere

## Notes

- Tasks run via `sh -c` on Unix — command strings are shell expressions
- Dep resolution uses topological sort with cycle detection
- The `generic` project type is the fallback when no language markers are found
- `pump` is deliberately generic over `Read`/`Write` so the tee can be unit-tested against in-memory buffers instead of racing two real pipes
- `run_streaming` writes through `io::stderr()` rather than a held `StderrLock`: holding the lock for the life of a stream would let one thread starve the other until its pipe closed, defeating the point of streaming
