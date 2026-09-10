---
spec: run.spec.md
---

## Test Plan

### Unit Tests

- `detect_project_type` correctly identifies rust, node, go, python, ruby, java-gradle, java-maven, swift, and generic projects
- `task_defaults` returns non-empty task maps for each supported project type
- Circular dependency detection catches direct cycles (A→B→A) and indirect cycles (A→B→C→A)
- Short-form task (`"cargo build"`) and full-form task (with deps, env, dir) both parse correctly
- `pump` mirrors and captures byte-identical content, forwards a partial line (a prompt with no newline), handles empty input, and handles payloads larger than its 8 KiB buffer
- `pump` keeps capturing to EOF after its mirror sink starts refusing writes, reports that failure rather than swallowing it, and stops mirroring at the first failure instead of retrying per chunk — while a *read* failure on the child's pipe still propagates as an error
- `stream_child` with a sink that refuses every write still returns the child's real exit code, its complete stdout/stderr, and a flagged mirror failure
- `join_pumps` joins the second forwarding thread even when the first one fails, so no thread is left detached
- `run_streaming` keeps stdout and stderr separate, propagates a non-zero exit code, retains output produced before a failure, and produces capture identical to `Command::output` for the same command

### Integration Tests

- `fledge run build` in a Rust project executes `cargo build`
- `fledge run --list` displays task names and descriptions from `fledge.toml`
- `fledge run --init` in a Rust project generates a valid `fledge.toml` with rust-specific tasks
- `fledge run nonexistent` fails with available task names listed
- Task with dependencies runs deps first in correct order
- `fledge run <task> --json` (default) keeps child output out of fledge's stderr and inside the envelope
- `fledge run <task> --json --stream` mirrors both child streams to fledge's stderr and still fills the envelope
- `fledge run <task> --json --stream` keeps child bytes off stdout: a dependency-free task leaves exactly one parseable envelope there even when the task prints JSON-shaped text
- A task with dependencies emits one envelope per executed task on stdout (pre-existing `--json` behaviour), each parseable and in dependency-first order, with no child bytes among them
- `fledge run <task> --json --stream` and the buffered run report the same `exit_code`, `success`, and captured stdout for a failing task
- `fledge run <task> --stream` (no `--json`) succeeds, shows child output, and keeps the `Running task:` summary; a failing one exits non-zero with the exit-code message
- Dependencies stream too when `--stream` is active

### Determinism Notes

- No test asserts cross-stream interleaving or relies on wall-clock timing. Streaming visibility is asserted by *presence* of child bytes on fledge's stderr after exit, which distinguishes the two modes without racing them
- The integration harness pipes stdout and stderr, so every streaming test also exercises the non-TTY path
