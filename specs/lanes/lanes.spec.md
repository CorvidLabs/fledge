---
module: lanes
version: 10
status: active
files:
  - src/lanes.rs

db_tables: []
depends_on:
  - run
  - config
  - github
  - search
---

# Lanes

## Purpose

Composable workflow pipelines defined in `fledge.toml` and `.fledge/lanes/`. Lanes chain multiple tasks (and inline commands) into named pipelines with support for parallel execution groups and configurable failure behavior. User-defined lanes live inline in `fledge.toml`; imported lanes are stored in `.fledge/lanes/*.toml`.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — dispatches lane actions |
| `LaneAction` | Enum: `Run`, `List`, `Init`, `Search`, `Import`, `Publish`, `Create`, `Validate` |
| `LaneDef` | Lane definition: description, steps, and fail_fast flag |

### Structs & Enums

| Type | Description |
|------|-------------|
| `LaneAction` | Action enum for the lane subcommand (Run with json/dry_run, List, Init, Search, Import, Publish, Create, Validate) |
| `LaneDef` | A named lane with description, steps, and fail_fast flag |
| `Step` | A single step: task reference, inline command, or parallel group |
| `ParallelItem` | An item within a parallel group: task reference or inline command |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(LaneAction) -> Result<()>` | Main entry — dispatch to init/list/execute/search/import |

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

# Parallel groups — items inside { parallel = [...] } run concurrently
# Items can be task references or inline commands
[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]

# Parallel with mixed types
[lanes.prep]
description = "Parallel tasks and inline commands"
steps = [
  { parallel = ["lint", { run = "echo checking" }] },
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
| Parallel group | `{ parallel = ["a", "b"] }` | Runs items concurrently (task refs or inline commands) |

## Invariants

1. Lanes are loaded from `fledge.toml` and merged with any `.fledge/lanes/*.toml` files; `fledge.toml` definitions take precedence
2. Each step in a lane is either a task reference (string), inline command (`{ run = "..." }`), or parallel group (`{ parallel = [...] }`) — parallel groups accept both task references and inline commands
3. Task references must resolve to tasks defined in `[tasks]` — unknown references produce an error before execution
4. Parallel groups spawn one thread per item via `std::thread::scope` and **wait for every sibling to finish** before reporting the group's result. There is no in-flight cancellation: a failure inside a parallel group does not interrupt its siblings, even when `fail_fast = true`. The `fail_fast` flag governs only what happens **after** the group completes — whether the lane proceeds to the next step or stops with the failure list. A thread panic is treated as a failure with the panic message captured.
5. Steps execute sequentially by default; only `{ parallel = [...] }` groups run concurrently
6. `fail_fast` defaults to `true` — when a step (sequential or parallel-group) reports failure, the lane stops before running the next step. With `fail_fast = false`, the lane runs every step regardless and reports the full failure list at the end
7. `--init` appends language-aware default lanes to an existing `fledge.toml`
8. `--dry-run` prints the execution plan without running anything
9. Task dependencies (deps) are resolved within each step — a task's deps run before the task itself
10. Each step prints its elapsed time on completion; the lane summary includes total elapsed time

## Behavioral Examples

```
# List lanes
$ fledge lanes
Available lanes:
  ci       Full CI pipeline
  release  Build and publish a release

# Run a lane (with step timing)
$ fledge lanes ci
▶️ Lane: ci — Full CI pipeline
  ▶️ Running task: lint
  ✔ Step 1 done (245ms)
  ▶️ Running task: test
  ✔ Step 2 done (1.032s)
  ▶️ Running task: build
  ✔ Step 3 done (3.456s)
✅ Lane ci completed (3 steps in 4.733s)

# Dry run
$ fledge lanes ci --dry-run
Lane: ci — Full CI pipeline
  1. lint (task)
  2. test (task)
  3. build (task)

# Parallel steps
$ fledge lanes run check
▶️ Lane: check — Quick quality check
  ▶️ Running parallel: lint, fmt
  ▶️ Running task: test
✅ Lane check completed (2 steps)

# Init default lanes
$ fledge lanes init
✅ Added default lanes to fledge.toml

# Search community lanes on GitHub
$ fledge lanes search
Community lanes on GitHub:
  CorvidLabs/fledge-lanes  (⭐ 12)  Official community lane collection
  user/rust-release-lane   (⭐ 3)   Rust release pipeline with cargo-dist

# Search with keyword
$ fledge lanes search rust
Community lanes on GitHub:
  user/rust-release-lane   (⭐ 3)   Rust release pipeline with cargo-dist

# Import lanes from a remote repo (saves to .fledge/lanes/)
$ fledge lanes import CorvidLabs/fledge-lanes
✅ Imported 3 lane(s) from CorvidLabs/fledge-lanes
  + release
  + deploy
  + audit
  → Saved to .fledge/lanes/corvidlabs-fledge-lanes.toml
  * Skipped (already exist): ci

# Import with version pinning
$ fledge lanes import CorvidLabs/fledge-lanes@v1.0.0

# Scaffold a lane repo
$ fledge lanes create my-lanes
✅ Created lane repo at ./my-lanes

# Validate lanes
$ fledge lanes validate
✅ . — valid (3 lanes)

# Validate with strict mode
$ fledge lanes validate --strict
.
  warn: Lane 'deploy' has no description
Validation failed

# Publish runs validation first
$ fledge lanes publish
✅ . — valid (2 lanes)
➡️ Publishing 2 lanes as owner/my-lanes

# Run a lane with JSON output
$ fledge lanes run ci --json
{"lane": "ci", "description": "Full CI pipeline", "total_steps": 3, "success": true, "duration_ms": 4733, "fail_fast": true, "steps": [...], "failures": []}
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
| Remote no fledge.toml | Import target has no fledge.toml | Error with message |
| Remote no lanes | Import target's fledge.toml has no lanes | Error with message |
| All lanes exist | All imported lanes already defined locally | Skip with message |
| Create dir exists | `create` target directory already exists | Error |
| Validate no fledge.toml | `validate` path missing fledge.toml | Error |
| Validate undefined task | Lane step references task not in `[tasks]` | Validation error |
| Validate empty steps | Lane has zero steps | Validation error |
| Validate empty parallel | Parallel group has no items | Validation error |
| Validate circular deps | Task deps form a cycle | Validation error |

## Dependencies

- `run` module (reuses task execution, project detection)
- `config` module (GitHub token for API auth)
- `github` module (GitHub API requests)
- `search` module (response parsing, URL encoding)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 10 | 2026-04-25 | Lock parallel-group + `fail_fast` semantics for 1.0: in-flight siblings always finish (no cancellation), `fail_fast` governs only the *next* step. Documents existing behavior — no code change |
| 9 | 2026-04-23 | Add `--json` flag to `lane run` for structured JSON output |
| 8 | 2026-04-22 | Add `create` and `validate` subcommands; `publish` now validates before pushing |
| 7 | 2026-04-21 | Imported lanes stored in `.fledge/lanes/` instead of appending to fledge.toml; lane loading merges both sources |
| 6 | 2026-04-21 | Add step timing — each step prints elapsed time, lane summary includes total |
| 5 | 2026-04-21 | Generalize parallel groups to accept inline commands, not just task refs |
| 4 | 2026-04-21 | Rename from flows to lanes — 1.0 branding |
| 3 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 2 | 2026-04-20 | Add community lane registry (search + import) |
| 1 | 2026-04-20 | Initial spec |
