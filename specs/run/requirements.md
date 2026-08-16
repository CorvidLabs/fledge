---
spec: run.spec.md
---

## User Stories

- As a developer, I want to run project tasks with `fledge run <task>` instead of remembering tool-specific commands
- As a developer, I want tasks to automatically run their dependencies first
- As a developer, I want to see all available tasks with `fledge run --list`
- As a developer, I want `fledge run --init` to generate a starter task file for my project type
- As a developer running a long task under `--json`, I want to see its progress while it runs instead of a silent wait followed by a wall of captured text
- As a developer running an interactive task, I want its prompts to reach my terminal and my keystrokes to reach the task
- As an agent or script, I want `fledge run <task> --json` to keep emitting exactly one JSON document on stdout no matter which output mode is active

## Acceptance Criteria

### REQ-run-001

The implementation SHALL meet this contract: `fledge run <task>` executes the named task from `fledge.toml`

### REQ-run-002

The implementation SHALL meet this contract: Task dependencies run in topological order before the requested task

### REQ-run-003

The implementation SHALL meet this contract: Circular dependencies produce an error listing the cycle

### REQ-run-004

The implementation SHALL meet this contract: `fledge run --list` shows task names and descriptions

### REQ-run-005

The implementation SHALL meet this contract: `fledge run --init` generates `fledge.toml` with defaults for the detected project type

### REQ-run-006

The implementation SHALL meet this contract: Unknown task names produce an error listing available tasks

### REQ-run-007

The implementation SHALL meet this contract: Tasks support environment variables and working directory overrides

### REQ-run-008

The implementation SHALL meet this contract: `fledge run <task> --json --stream` forwards the child's stdout and stderr while the task runs, mirroring them to fledge's stderr, and still reports both streams in full in the `run_task` envelope

### REQ-run-009

The implementation SHALL meet this contract: `--stream` is opt-in. Without it, `--json` runs stay buffered and no child bytes appear on fledge's stderr

### REQ-run-010

The implementation SHALL meet this contract: with `--stream --json`, fledge's stdout contains exactly one parseable JSON envelope, even when the task itself writes JSON-shaped output

### REQ-run-011

The implementation SHALL meet this contract: exit codes and the failure message are identical with and without `--stream`, and the envelope's `exit_code`/`success` reflect the child's real status

### REQ-run-012

The implementation SHALL meet this contract: `--stream` forwards output whether or not the destination is a TTY, and forwards bytes verbatim

### REQ-run-013

The implementation SHALL meet this contract: `--stream` without `--json` is accepted and leaves behaviour unchanged (that path already inherits the terminal)

## Constraints

- Tasks execute via `sh -c` — must work on macOS and Linux
- `fledge.toml` must be present in the current directory (or `--init` to create one)
- `--stream` may not alter the `run_task` envelope's field set or `schema_version` — it is a presentation mode, not a contract change
- Cross-stream (stdout vs stderr) ordering cannot be guaranteed when both are captured separately; only per-stream ordering is promised

## Acceptance and Rejection Signals

**Accepted when:**

- `run <task> --json --stream` emits child bytes on fledge's stderr before the process exits, and the envelope on stdout still parses and carries the same `stdout`/`stderr` content as a buffered run of the same task
- `run <task> --json` (no `--stream`) emits no child bytes on fledge's stderr
- Both modes report the same `exit_code` and the same non-zero-exit error message
- `run <task> --stream` (no `--json`) succeeds and prints the usual `Running task: <name>` summary

**Rejected when:**

- `--stream --json` writes child output to stdout, or stdout stops being a single JSON document
- `--stream --json` leaves `stdout`/`stderr` empty or truncated in the envelope
- The buffered default starts streaming, or the envelope gains/loses fields
- Forwarding is skipped because the destination is not a TTY
- An exit code differs between the two modes

## Out of Scope

- Parallel task execution
- Task caching or incremental builds
- Watch mode / file-change triggers
- Guaranteed cross-stream interleaving under `--stream --json`
- Streaming for `lanes` steps (a separate module)

### REQ-run-020

`fledge run` SHALL accept an opt-in `--stream` flag that forwards child stdout and
stderr live on the `--json` execution path, which otherwise buffers both pipes until the
task exits.

Acceptance Criteria
- With `--stream --json`, child bytes appear on fledge's stderr while the task is still running.
- Without `--stream`, capture remains buffered — the default behavior is unchanged.
- `--stream` without `--json` is accepted as a documented no-op and preserves the task summary.
- `--stream` propagates to dependency tasks.

### REQ-run-021

Streaming SHALL NOT weaken the `--json` envelope.

Acceptance Criteria
- The envelope's `stdout` and `stderr` fields are populated identically to the buffered path.
- The envelope field set and `schema_version` are unchanged.
- For the same failing task, the streamed and buffered envelopes are equal, including `exit_code`.
- Child bytes are mirrored to stderr, never to stdout, so stdout remains a single JSON document.

### REQ-run-022

Streaming SHALL forward each stream in order and inherit stdin.

Acceptance Criteria
- Each of stdout and stderr is forwarded in order, and a chunk is never split by the other stream.
- Cross-stream interleaving is best-effort and documented as such; it is not guaranteed.
- The child inherits stdin under `--stream`, so an interactive task can prompt — unlike the
  buffered path, which closes stdin.
- Output is forwarded verbatim, with no colour, prefixes or line framing added.

