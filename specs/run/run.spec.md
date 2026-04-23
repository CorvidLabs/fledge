---
module: run
version: 2
status: active
files:
  - src/run.rs

db_tables: []
depends_on: []
---

# Run

## Purpose

Task runner that reads task definitions from `fledge.toml` and executes them. Supports simple string commands, full task configs with dependencies, environment variables, and working directory overrides. When no `fledge.toml` exists, auto-detects the project type and synthesizes tasks in memory. Only suggests `fledge run --init` for unrecognized (generic) project types.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — lists or executes tasks |
| `RunOptions` | Options: `task`, `init`, `list` |
| `detect_project_type` | Detects project ecosystem from directory contents |
| `task_defaults` | Returns default task definitions for a given project type |
| `detect_node_runner` | Detects node package manager from lock files (bun, yarn, pnpm, npm) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `RunOptions` | Options: `task`, `init`, `list`, `lang`, `json` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(RunOptions) -> Result<()>` | Main entry — dispatch to init/list/execute |
| `detect_project_type` | `(&Path) -> &'static str` | Detect project ecosystem (rust, node, go, python, etc.) from marker files |
| `detect_node_runner` | `(&Path) -> &'static str` | Detect Node.js package manager (npm, bun, pnpm, yarn) |
| `task_defaults` | `(&str, &Path) -> String` | Return default task TOML entries for a given project type and directory |
| `detect_node_runner` | `(&Path) -> &'static str` | Detect node package manager (bun, yarn, pnpm, npm) from lock files in directory |

## Invariants

1. Tasks are read from `fledge.toml` in the current directory, or auto-detected from project type when no `fledge.toml` exists
2. Short-form tasks (`name = "cmd"`) and full-form (`[tasks.name]` with `cmd`, `deps`, `description`, `env`, `dir`) are both supported
3. Dependencies are executed before the task itself
4. Circular dependencies are detected and produce an error
5. `--init` creates a starter `fledge.toml` if none exists
6. When auto-detecting, the task list header indicates tasks are auto-detected and suggests creating `fledge.toml` to customize
7. `--json` outputs structured JSON for both task listing and task execution
8. `--lang` overrides auto-detected project type (e.g. `rust`, `node`, `go`, `python`, `swift`, `ruby`, `java-gradle`, `java-maven`)

## Behavioral Examples

```
# List tasks
$ fledge run
Available tasks:
  build  cargo build
  test   cargo test

# Run a task
$ fledge run build
▶️ Running task: build

# Run a task with dependencies
$ fledge run ci
▶️ Running task: lint
▶️ Running task: ci

# Init a new fledge.toml
$ fledge run --init
✓ Created fledge.toml

# List tasks as JSON
$ fledge run --json
{"auto_detected": false, "tasks": [...]}

# Run a task with JSON output
$ fledge run test --json
{"task": "test", "command": "cargo test", "exit_code": 0, "success": true, "stdout": "...", "stderr": "..."}

# Override project type
$ fledge run --lang node
Available tasks:
  build  npm run build
  test   npm test
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No fledge.toml (generic project) | File missing and project type is unrecognized | Suggest `fledge run --init` |
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
| 2 | 2026-04-23 | Add `--json` flag (list + execute), `--lang` override, `detect_node_runner` |
| 1 | 2026-04-19 | Initial spec |
