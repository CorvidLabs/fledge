---
spec: run.spec.md
---

## User Stories

- As a developer, I want to run project tasks with `fledge run <task>` instead of remembering tool-specific commands
- As a developer, I want tasks to automatically run their dependencies first
- As a developer, I want to see all available tasks with `fledge run --list`
- As a developer, I want `fledge run --init` to generate a starter task file for my project type

## Acceptance Criteria

- `fledge run <task>` executes the named task from `fledge.toml`
- Task dependencies run in topological order before the requested task
- Circular dependencies produce an error listing the cycle
- `fledge run --list` shows task names and descriptions
- `fledge run --init` generates `fledge.toml` with defaults for the detected project type
- Unknown task names produce an error listing available tasks
- Tasks support environment variables and working directory overrides

## Constraints

- Tasks execute via `sh -c` — must work on macOS and Linux
- `fledge.toml` must be present in the current directory (or `--init` to create one)

## Out of Scope

- Parallel task execution
- Task caching or incremental builds
- Watch mode / file-change triggers
