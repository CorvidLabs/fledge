---
module: run
version: 1
status: active
files:
  - src/run.rs

db_tables: []
depends_on: []
---

# Run

## Purpose

Task runner that reads task definitions from `fledge.toml` and executes them. Supports simple string commands, full task configs with dependencies, environment variables, and working directory overrides.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — lists or executes tasks |
| `RunOptions` | Options: `task`, `init`, `list` |
| `detect_project_type` | Detects project type from directory contents (Cargo.toml → rust, package.json → node, etc.) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `RunOptions` | Options: `task`, `init`, `list` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(RunOptions) -> Result<()>` | Main entry — dispatch to init/list/execute |
| `detect_project_type` | `(&Path) -> &'static str` | Returns project type string based on marker files in directory |

## Invariants

1. Tasks are read from `fledge.toml` in the current directory
2. Short-form tasks (`name = "cmd"`) and full-form (`[tasks.name]` with `cmd`, `deps`, `env`, `dir`, `description`) are both supported
3. Dependencies are executed before the task itself
4. Circular dependencies are detected and produce an error
5. `--init` creates a starter `fledge.toml` if none exists

## Behavioral Examples

```
# List tasks
$ fledge run
Available tasks:
  build  cargo build
  test   cargo test

# Run a task
$ fledge run build
▸ Running task: build

# Run a task with dependencies
$ fledge run ci
▸ Running task: lint
▸ Running task: ci

# Init a new fledge.toml
$ fledge run --init
✓ Created fledge.toml
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No fledge.toml | File missing | Suggest `fledge run --init` |
| No tasks defined | Empty `[tasks]` section | Error with guidance |
| Unknown task | Task name not found | List available tasks |
| Circular dependency | Task A depends on B depends on A | Error with cycle info |
| Task failed | Non-zero exit code | Error with exit code |
| Already exists | `--init` when fledge.toml exists | Error |

## Dependencies

- None (uses only std and serde/toml/console)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec |
