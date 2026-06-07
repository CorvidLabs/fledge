---
module: run
version: 5
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
| `RunOptions` | Options: `task`, `init`, `list`, `lang`, `json`, `args` |

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
9. Arguments after a `--` separator are passed through to the target task's command. They apply to the named task only — dependencies always run without them
10. Pass-through is safe by construction: on POSIX the args become real shell positional parameters (`sh -c '<cmd> "$@"' fledge <args…>`), never interpolated into the command string. `"$@"` is auto-appended unless the command already references a positional (`$1`..`$9`, `$@`, `$*`, or their `${…}` forms), in which case the args fill those positionals without being doubled. With no pass-through args the invocation is identical to before the feature. On Windows (`cmd /C`) there is no `$@`; args are appended as argv (best-effort)
11. `run <task> --json` includes an `args` array in the envelope only when pass-through args were supplied; arg-less runs keep their prior envelope shape

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
{"schema_version": 1, "action": "run_list", "auto_detected": false, "tasks": [...]}

# Init fledge.toml as JSON
$ fledge run --init --json
{"schema_version": 1, "action": "run_init", "file": "fledge.toml", "project_type": "rust", "files_created": ["fledge.toml"]}

# Run a task with JSON output
$ fledge run test --json
{"schema_version": 1, "action": "run_task", "task": "test", "command": "cargo test", "exit_code": 0, "success": true, "stdout": "...", "stderr": "..."}

# Pass arguments through to the task command (after `--`)
$ fledge run test -- --nocapture --test-threads=1
▶️ Running task: test
# → runs: cargo test --nocapture --test-threads=1

# Pass a value through (e.g. a version)
$ fledge run set-version -- 1.2.3
# → runs: ./set-version.sh 1.2.3

# Pass-through with JSON adds an `args` array to the envelope
$ fledge run test --json -- --nocapture
{"schema_version": 1, "action": "run_task", "task": "test", "command": "cargo test", "exit_code": 0, "success": true, "stdout": "...", "stderr": "...", "args": ["--nocapture"]}

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
| 5 | 2026-06-07 | Add task argument pass-through: `fledge run <task> -- <args…>` forwards args to the target task's command (named task only, not deps). POSIX uses real positional params (`"$@"`, auto-appended unless the command references `$1`/`$@`/…), so values are never interpolated into the command string — no injection surface. `--json` gains an `args` array when args are supplied. Additive and backward-compatible: arg-less runs are byte-identical to before. New `references_positional`/`build_task_command` helpers with unit + injection-safety tests |
| 4 | 2026-04-26 | Doc sync, behavioral examples updated to show the post-tier-D envelope shapes for `run --json`, `run <task> --json`, and `run --init --json`. No code change |
| 3 | 2026-04-26 | Tier-D 1.0 envelope: all three `--json` paths now emit `{schema_version: 1, action, ...}`. `run --init --json` previously emitted prose ("✅ Created fledge.toml"), now `{action: "run_init", file, project_type, files_created}`, a real fix not just a wrapping. `run --list --json` adds `action: "run_list"` (was bare `{auto_detected, tasks}`). `run <task> --json` adds `action: "run_task"` (was bare `{task, command, ...}`). Three new integration tests guard each shape |
| 2 | 2026-04-23 | Add `--json` flag (list + execute), `--lang` override, `detect_node_runner` |
| 1 | 2026-04-19 | Initial spec |
