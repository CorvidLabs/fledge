---
module: lanes
version: 1
status: active
files:
  - src/lanes.rs

db_tables: []
depends_on:
  - run
---

# Lanes

## Purpose

Composable workflow pipelines defined in `fledge.toml`. Lanes chain multiple tasks (and inline commands) into named pipelines with support for parallel execution groups and configurable failure behavior.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — lists or executes lanes |
| `LaneOptions` | Options: `lane`, `list`, `init`, `dry_run`, `json` |
| `LaneDef` | Lane definition: description, steps, and fail_fast flag |

### Structs & Enums

| Type | Description |
|------|-------------|
| `LaneOptions` | CLI options for the lane subcommand |
| `LaneDef` | A named lane with description, steps, and fail_fast flag |
| `Step` | A single step: task reference, inline command, or parallel group |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(LaneOptions) -> Result<()>` | Main entry — dispatch to init/list/execute |

## Config Format

Lanes are defined in `fledge.toml` under `[lanes]`:

```toml
# Sequential pipeline — steps reference tasks by name
[lanes.ci]
description = "Full CI pipeline"
steps = ["lint", "test", "build"]

# Mixed steps — task references and inline commands
[lanes.release]
description = "Build and publish a release"
steps = [
  "test",
  { run = "cargo build --release" },
  "publish"
]

# Parallel groups — steps inside { parallel = [...] } run concurrently
[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]

# Failure behavior — fail_fast = false continues after failures
[lanes.audit]
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

1. Lanes are read from `fledge.toml` alongside tasks
2. Each step in a lane is either a task reference (string), inline command (`{ run = "..." }`), or parallel group (`{ parallel = [...] }`)
3. Task references must resolve to tasks defined in `[tasks]` — unknown references produce an error before execution
4. Parallel groups spawn threads and collect results; if any thread fails and `fail_fast` is true, remaining steps are skipped
5. Steps execute sequentially by default; only `{ parallel = [...] }` groups run concurrently
6. `fail_fast` defaults to `true` — first failure stops the lane
7. `--init` appends language-aware default lanes to an existing `fledge.toml`
8. `--dry-run` prints the execution plan without running anything
9. Task dependencies (deps) are resolved within each step — a task's deps run before the task itself

## Behavioral Examples

```
# List lanes
$ fledge lane
Available lanes:
  ci       Full CI pipeline
  release  Build and publish a release

# Run a lane
$ fledge lane ci
▸ Lane: ci — Full CI pipeline
  ▸ Running task: lint
  ▸ Running task: test
  ▸ Running task: build
✓ Lane ci completed (3 steps)

# Dry run
$ fledge lane ci --dry-run
Lane: ci — Full CI pipeline
  1. lint (task)
  2. test (task)
  3. build (task)

# Parallel steps
$ fledge lane check
▸ Lane: check — Quick quality check
  ▸ Running parallel: lint, fmt
  ▸ Running task: test
✓ Lane check completed (2 steps)

# Init default lanes
$ fledge lane --init
✓ Added default lanes to fledge.toml
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No fledge.toml | File missing | Suggest `fledge run --init` |
| No lanes defined | No `[lanes]` section | Error with guidance |
| Unknown lane | Lane name not found | List available lanes |
| Unknown task ref | Step references non-existent task | Error before execution with task name |
| Step failed | Non-zero exit code | Stop lane (if fail_fast) or continue and report |
| Already exists | `--init` when lanes already exist | Error |
| Empty steps | Lane has no steps | Error |

## Dependencies

- `run` module (reuses task execution, project detection)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
