---
spec: run.spec.md
---

## User Stories

- As a developer, I want to run project tasks with `fledge run <task>` instead of remembering tool-specific commands
- As a developer, I want tasks to automatically run their dependencies first
- As a developer, I want to see all available tasks with `fledge run --list`
- As a developer, I want `fledge run --init` to generate a starter task file for my project type

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

## Constraints

- Tasks execute via `sh -c` — must work on macOS and Linux
- `fledge.toml` must be present in the current directory (or `--init` to create one)

## Out of Scope

- Parallel task execution
- Task caching or incremental builds
- Watch mode / file-change triggers
