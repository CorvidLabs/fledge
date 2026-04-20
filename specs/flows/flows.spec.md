---
module: flows
version: 1
status: active
files:
  - src/flows.rs

db_tables: []
depends_on:
  - run
---

# Flows

## Purpose

Composable workflow pipelines defined in `fledge.toml`. Flows chain multiple tasks (and inline commands) into named pipelines with support for parallel execution groups and configurable failure behavior.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — lists or executes lanes |
| `FlowOptions` | Options: `flow`, `list`, `init`, `dry_run`, `json` |
| `FlowDef` | Flow definition: description, steps, and fail_fast flag |

### Structs & Enums

| Type | Description |
|------|-------------|
| `FlowOptions` | CLI options for the flow subcommand |
| `FlowDef` | A named flow with description, steps, and fail_fast flag |
| `Step` | A single step: task reference, inline command, or parallel group |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(FlowOptions) -> Result<()>` | Main entry — dispatch to init/list/execute |

## Config Format

Flows are defined in `fledge.toml` under `[flows]`:

```toml
# Sequential pipeline — steps reference tasks by name
[flows.ci]
description = "Full CI pipeline"
steps = ["lint", "test", "build"]

# Mixed steps — task references and inline commands
[flows.release]
description = "Build and publish a release"
steps = [
  "test",
  { run = "cargo build --release" },
  "publish"
]

# Parallel groups — steps inside { parallel = [...] } run concurrently
[flows.check]
description = "Quick quality check"
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]

# Failure behavior — fail_fast = false continues after failures
[flows.audit]
description = "Run all audits"
fail_fast = false
steps = ["deps-audit", "license-check", "security-scan"]
```

### Step Types

| Type | Format | Description |
|------|--------|-------------|
| Task reference | `"task_name"` | Runs a task defined in `[tasks]` |
| Inline command | `{ run = "command" }` | Runs a shell command directly |
| Parallel group | `{ parallel = ["a", "b"] }` | Runs referenced tasks concurrently |

## Invariants

1. Flows are read from `fledge.toml` alongside tasks
2. Each step in a flow is either a task reference (string), inline command (`{ run = "..." }`), or parallel group (`{ parallel = [...] }`)
3. Task references must resolve to tasks defined in `[tasks]` — unknown references produce an error before execution
4. Parallel groups spawn threads and collect results; if any thread fails and `fail_fast` is true, remaining steps are skipped
5. Steps execute sequentially by default; only `{ parallel = [...] }` groups run concurrently
6. `fail_fast` defaults to `true` — first failure stops the flow
7. `--init` appends language-aware default flows to an existing `fledge.toml`
8. `--dry-run` prints the execution plan without running anything
9. Task dependencies (deps) are resolved within each step — a task's deps run before the task itself

## Behavioral Examples

```
# List lanes
$ fledge flow
Available lanes:
  ci       Full CI pipeline
  release  Build and publish a release

# Run a flow
$ fledge flow ci
▸ Flow: ci — Full CI pipeline
  ▸ Running task: lint
  ▸ Running task: test
  ▸ Running task: build
✓ Flow ci completed (3 steps)

# Dry run
$ fledge flow ci --dry-run
Flow: ci — Full CI pipeline
  1. lint (task)
  2. test (task)
  3. build (task)

# Parallel steps
$ fledge flow check
▸ Flow: check — Quick quality check
  ▸ Running parallel: lint, fmt
  ▸ Running task: test
✓ Flow check completed (2 steps)

# Init default lanes
$ fledge flow --init
✓ Added default flows to fledge.toml
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No fledge.toml | File missing | Suggest `fledge run --init` |
| No lanes defined | No `[flows]` section | Error with guidance |
| Unknown flow | Flow name not found | List available flows |
| Unknown task ref | Step references non-existent task | Error before execution with task name |
| Step failed | Non-zero exit code | Stop flow (if fail_fast) or continue and report |
| Already exists | `--init` when lanes already exist | Error |
| Empty steps | Flow has no steps | Error |

## Dependencies

- `run` module (reuses task execution, project detection)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
