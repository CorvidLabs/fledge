---
module: flows
version: 3
status: active
files:
  - src/flows.rs

db_tables: []
depends_on:
  - run
  - config
  - github
  - search
---

# Flows

## Purpose

Composable workflow pipelines defined in `fledge.toml`. Flows chain multiple tasks (and inline commands) into named pipelines with support for parallel execution groups and configurable failure behavior.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — dispatches flow actions |
| `FlowAction` | Enum: `Run`, `List`, `Init`, `Search`, `Import` |
| `FlowDef` | Flow definition: description, steps, and fail_fast flag |

### Structs & Enums

| Type | Description |
|------|-------------|
| `FlowAction` | Action enum for the flow subcommand (Run, List, Init, Search, Import) |
| `FlowDef` | A named flow with description, steps, and fail_fast flag |
| `Step` | A single step: task reference, inline command, or parallel group |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(FlowAction) -> Result<()>` | Main entry — dispatch to init/list/execute/search/import |

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
# List flows
$ fledge flow
Available flows:
  ci       Full CI pipeline
  release  Build and publish a release

# Run a flow
$ fledge flow ci
▶️ Flow: ci — Full CI pipeline
  ▶️ Running task: lint
  ▶️ Running task: test
  ▶️ Running task: build
✅ Flow ci completed (3 steps)

# Dry run
$ fledge flow ci --dry-run
Flow: ci — Full CI pipeline
  1. lint (task)
  2. test (task)
  3. build (task)

# Parallel steps
$ fledge flow run check
▶️ Flow: check — Quick quality check
  ▶️ Running parallel: lint, fmt
  ▶️ Running task: test
✅ Flow check completed (2 steps)

# Init default flows
$ fledge flow init
✅ Added default flows to fledge.toml

# Search community flows on GitHub
$ fledge flow search
Community flows on GitHub:
  CorvidLabs/fledge-flows  (⭐ 12)  Official community flow collection
  user/rust-release-flow   (⭐ 3)   Rust release pipeline with cargo-dist

# Search with keyword
$ fledge flow search rust
Community flows on GitHub:
  user/rust-release-flow   (⭐ 3)   Rust release pipeline with cargo-dist

# Import flows from a remote repo
$ fledge flow import CorvidLabs/fledge-flows
✅ Imported 3 flow(s) from CorvidLabs/fledge-flows
  + release
  + deploy
  + audit
  + Also added 2 task(s): package, upload
  * Skipped (already exist): ci

# Import with version pinning
$ fledge flow import CorvidLabs/fledge-flows@v1.0.0
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No fledge.toml | File missing | Suggest `fledge run --init` |
| No flows defined | No `[flows]` section | Error with guidance |
| Unknown flow | Flow name not found | List available flows |
| Unknown task ref | Step references non-existent task | Error before execution with task name |
| Step failed | Non-zero exit code | Stop flow (if fail_fast) or continue and report |
| Already exists | `--init` when flows already exist | Error |
| Empty steps | Flow has no steps | Error |
| Remote no fledge.toml | Import target has no fledge.toml | Error with message |
| Remote no flows | Import target's fledge.toml has no flows | Error with message |
| All flows exist | All imported flows already defined locally | Skip with message |

## Dependencies

- `run` module (reuses task execution, project detection)
- `config` module (GitHub token for API auth)
- `github` module (GitHub API requests)
- `search` module (response parsing, URL encoding)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 2 | 2026-04-20 | Add community flow registry (search + import) |
| 1 | 2026-04-20 | Initial spec |
