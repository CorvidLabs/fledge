---
module: lanes
version: 22
status: active
files:
  - src/lanes/mod.rs
  - src/lanes/community.rs
  - src/lanes/create.rs
  - src/lanes/defaults.rs
  - src/lanes/execute.rs
  - src/lanes/publish.rs
  - src/lanes/validate.rs
  - src/lanes/tests.rs

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
| `run_for_pre_release` | Silent lane gate for `release --json --pre-lane`; runs steps with subprocess output suppressed and emits no envelope. Failure bails to stderr |
| `LaneAction` | Enum: `Run`, `List`, `Init`, `Search`, `Import`, `Publish`, `Create`, `Validate` |
| `LaneDef` | Lane definition: description, steps, and fail_fast flag |
| `LaneValidationReport` | Validation results struct: path, lane count, errors, warnings |
| `search_lanes` | Search GitHub for community lanes matching keyword/author filters |
| `import_lanes` | Import lanes from a remote GitHub repository into local config |
| `parse_import_source` | Parse import source string into owner, repo, subpath, git ref |
| `base64_decode` | Decode base64 string (standard and no-pad variants) |
| `create_lane_repo` | Scaffold a new lane repository template |
| `lane_defaults` | Return project-type-specific default lane TOML |
| `init_lanes` | Initialize default lanes in fledge.toml based on detected project type |
| `execute_lane` | Execute a lane with progress display (human-readable or JSON) |
| `execute_lane_silent` | Execute a lane without console output |
| `execute_task_with_deps` | Execute a task and all its dependencies with cycle detection |
| `publish_lanes` | Publish a lane repository to GitHub with fledge-lane topic |
| `validate_lanes` | Validate lanes for structural integrity and task references |
| `print_lane_report` | Output a validation report in human-readable or JSON format |

### Structs & Enums

| Type | Description |
|------|-------------|
| `LaneAction` | Action enum for the lane subcommand (Run with json/dry_run/from, List, Init, Search, Import, Publish, Create, Validate) |
| `LaneDef` | A named lane with description, steps, and fail_fast flag |
| `Step` | A single step: task reference (bare string or `{ task = "..." }`), inline command (`{ run = "..." }`), or parallel group (`{ parallel = [...] }`). Table-form steps support optional `when`, `timeout`, and `retries` fields |
| `ParallelItem` | An item within a parallel group: task reference or inline command |
| `LaneValidationReport` | Validation results: path, lane_count, errors, warnings (serializable) |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(LaneAction) -> Result<()>` | Main entry — dispatch to init/list/execute/search/import |
| `run_for_pre_release` | `(&str, bool) -> Result<()>` | Run a lane silently as a pre-release gate |
| `search_lanes` | `(Option<&str>, Option<&str>, bool) -> Result<()>` | Search GitHub for community lanes matching keyword and author filters |
| `import_lanes` | `(&str, bool, bool) -> Result<()>` | Import lanes from a remote GitHub repo into local `.fledge/lanes/` |
| `parse_import_source` | `(&str) -> (String, String, Option<String>, Option<String>)` | Parse import source into (owner, repo, subpath, git_ref) |
| `base64_decode` | `(&str) -> Result<Vec<u8>>` | Decode base64 string (standard and no-pad) |
| `create_lane_repo` | `(&str, &Path, Option<&str>, bool, bool) -> Result<()>` | Scaffold a new lane repository with fledge.toml, README, .gitignore |
| `lane_defaults` | `(&str) -> &'static str` | Return default lane TOML for a project type |
| `init_lanes` | `(bool) -> Result<()>` | Initialize default lanes in fledge.toml based on detected project type |
| `execute_lane` | `(&str, &LaneDef, &BTreeMap<String, TaskDef>, &Path, bool, Option<usize>) -> Result<()>` | Execute a lane with human-readable or JSON progress, optional `--from` index |
| `execute_lane_silent` | `(&str, &LaneDef, &BTreeMap<String, TaskDef>, &Path) -> Result<()>` | Execute a lane without console output |
| `execute_task_with_deps` | `(&str, &BTreeMap<String, TaskDef>, &Path, bool, Option<Instant>) -> Result<()>` | Execute a task and all its dependencies with cycle detection, optional deadline |
| `resolve_from` | `(&[Step], &str) -> Result<usize>` | Resolve `--from` argument to 0-based step index (by name or 1-based index) |
| `evaluate_when` | `(&str) -> bool` | Evaluate a `when` condition string against environment variables |
| `publish_lanes` | `(&Path, Option<&str>, bool, Option<&str>, bool, bool) -> Result<()>` | Publish lane repo to GitHub with fledge-lane topic |
| `validate_lanes` | `(&Path, bool, bool) -> Result<()>` | Validate lanes for structural integrity, task refs, circular deps |
| `print_lane_report` | `(&LaneValidationReport, bool, bool) -> Result<()>` | Output validation report in human-readable or JSON format |

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
| Task reference (full) | `{ task = "name" }` | Runs a task with optional `when`/`timeout`/`retries` |
| Inline command | `{ run = "command" }` | Runs a shell command directly |
| Parallel group | `{ parallel = ["a", "b"] }` | Runs items concurrently (task refs or inline commands) |

### Step Options (table-form steps only)

| Option | Type | Description |
|--------|------|-------------|
| `when` | `string` | Condition: skip step if not met. Supports `VAR` (set & non-empty), `VAR=value` (equals), `!VAR` (not set/empty), `!VAR=value` (not equals). Multiple comma-separated conditions are AND'd |
| `timeout` | `integer` | Per-attempt timeout in seconds. Each retry attempt gets a fresh deadline. The deadline covers the entire step including task deps. Process is killed on timeout |
| `retries` | `integer` | Number of retry attempts after failure (0 = no retries, default). Total attempts = retries + 1. A 1-second delay is inserted between attempts |

```toml
# Examples of step options
steps = [
  { task = "deploy", when = "CI=true", timeout = 120 },
  { run = "curl health", retries = 3, timeout = 10 },
  { parallel = ["lint", "fmt"], when = "!SKIP_CHECKS" },
]
```

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
11. `--from <step>` skips all steps before the target (by 1-based index or step name). Stateless — no run history is persisted. Skipped steps show in both human-readable and JSON output
12. `when` conditions are evaluated against environment variables before each step executes. Steps with unmet conditions are silently skipped (shown as skipped in output). Evaluation is AND-logic for comma-separated conditions
13. `timeout` sets a per-attempt deadline in seconds. Each retry attempt gets a fresh deadline. The deadline covers the entire step execution including task dependency resolution. The direct child process is killed when the deadline passes; grandchild processes spawned by shell commands (e.g. `sh -c "..."`) are not tracked and may continue running as orphans
14. `retries` specifies the number of retry attempts after failure. Total attempts = retries + 1. Retry is per-step (the full step re-executes, not individual commands within it). A 1-second delay is inserted between retry attempts to avoid hammering failing resources

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

# Resume from a specific step (skip earlier ones)
$ fledge lanes run ci --from 3
▶️ Lane: ci — Full CI pipeline
  ⚙ starting from step 3
  ⏭ Step 1 (skipped by --from)
  ⏭ Step 2 (skipped by --from)
  ▶️ Running task: build
  ✔ Step 3 done (3.456s)
✅ Lane ci completed (3 steps in 3.456s)

# Resume by step name
$ fledge lanes run ci --from test

# Conditional step skipped when condition not met
$ fledge lanes run release
▶️ Lane: release — Build and publish
  ▶️ Running task: test
  ✔ Step 1 done (1.032s)
  ⏭ Step 2 deploy (skipped: when 'CI=true' not met)
✅ Lane release completed (2 steps in 1.032s)

# Run a lane with JSON output (each step record has step/name/success/duration_ms/error)
$ fledge lanes run ci --json
{"schema_version": 2, "lane": "ci", "description": "Full CI pipeline", "total_steps": 3, "success": true, "duration_ms": 4733, "fail_fast": true, "steps": [{"step": 1, "name": "lint", "success": true, "duration_ms": 245, "error": null}, ...], "failures": []}

# Dry-run with JSON (plan only — no duration_ms, includes dry_run: true)
$ fledge lanes run ci --json --dry-run
{"schema_version": 2, "lane": "ci", "description": "Full CI pipeline", "total_steps": 3, "fail_fast": true, "dry_run": true, "steps": [{"step": 1, "kind": "task", "name": "lint"}, {"step": 2, "kind": "parallel", "items": [{"kind": "task", "name": "fmt"}]}, ...]}
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

## Compatibility Policy

`lanes v1` is the stable schema that ships with fledge 1.0. To protect
project authors and lane-repo publishers from breakage, the following rules
govern how the lane TOML schema and `LaneDef`/`Step`/`ParallelItem` Rust types
may evolve within the v1 major version:

1. **Additive-only step kinds.** The current `Step` variants — `TaskRef` (bare string), `Inline` (`{ run = "..." }`), and `Parallel` (`{ parallel = [...] }`) — are locked. New step kinds may be added at any time, but each must use a unique discriminator key in the inline-table form (e.g. `{ matrix = [...] }`, `{ when = "..." }`) so the existing untagged enum continues to deserialize older lanes unchanged.
2. **No field removal.** Once shipped, every field on `[lanes.<name>]` (`description`, `steps`, `fail_fast`) and on each `[tasks.<name>]` (`cmd`, `deps`, `env`, `dir`) must continue to be parsed. Removing a field is a breaking change and requires a new schema version.
3. **No field retyping.** A field's TOML type is locked once shipped. The bare-string form of `TaskRef` and the inline-table forms of `Inline`/`Parallel` are part of the wire contract and cannot change shape.
4. **New optional fields are allowed.** Both fledge and lane consumers must tolerate unknown fields on known sections — additive optional fields do not require a version bump.
5. **Parallel + fail_fast semantics are locked.** In-flight siblings always finish (no cancellation). `fail_fast` governs only what happens *after* a step or group completes — never mid-execution. Documented in invariant 4.
6. **`fail_fast` defaults to `true`.** Changing the default is a breaking change.
7. **Trust-tier classification is part of the contract.** A source string ending in `<owner>/<repo>` (with optional `@<ref>`) is the canonical form. Future variations (multi-host, hashed pins) must classify under the existing `Official | Team | Unverified` tiers.
8. **`--json` envelope shapes are versioned per-command.** Adding fields is additive within a given command's `schema_version`. Renaming or removing fields bumps that command's `schema_version` (see `LANES_*_SCHEMA` constants).
9. **Imported lane file naming is locked.** `.fledge/lanes/<safe>.toml` with the `# Imported from <source>` first-line marker is part of the v1 contract — fledge uses both for trust classification and for round-trip integrity.

Any change that cannot be expressed under these rules requires a new schema
version declared explicitly (e.g. `[fledge] lanes_version = 2`); v1 lane
files continue to load against v1 semantics indefinitely.

## Dependencies

- `run` module (reuses task execution, project detection)
- `config` module (GitHub token for API auth)
- `github` module (GitHub API requests)
- `search` module (response parsing, URL encoding)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 22 | 2026-05-04 | Review fixes: (a) 1-second retry delay between attempts. (b) Timeout is per-attempt — each retry gets a fresh deadline. (c) Bump `LANES_RUN_SCHEMA` and `LANES_DRY_RUN_SCHEMA` from 1 → 2 for new fields. (d) Fix spec: remove private fns from Exported Functions, document process tree limitation on timeout |
| 21 | 2026-05-04 | Add `when` (conditional steps), `timeout` (per-step deadlines), `retries`, and `--from` (resume from step). New `Step` options on table-form steps. New exports: `resolve_from`, `evaluate_when`. Invariants 11-14 |
| 20 | 2026-05-01 | **1.0 contract finalize:** Add Compatibility Policy locking the lanes v1 schema. Future Step variants must use unique discriminator keys in inline-table form so the untagged enum keeps deserializing older lanes. Field removal/retyping bumps schema; parallel + fail_fast semantics, fail_fast default, imported-lane file naming, and trust-tier classification are all locked. No code change |
| 19 | 2026-04-29 | Document all submodule exports (community, create, defaults, execute, publish, validate) after splitting lanes.rs into lanes/ module folder |
| 18 | 2026-04-27 | **Breaking (1.0 contract finalize, follow-up):** `lanes publish --json` cancelled and success paths now share the same key set (`schema_version`, `action`, `cancelled`, `repo`, `lanes_published`, `topic`, `import_hint`); `cancelled: true` when the user declines, `false` on success. The cancelled `repo.exists` field is removed (`created: false` covers it). Mirrors the same fix landed for `plugins publish` and `templates publish` in the previous commit |
| 17 | 2026-04-26 | **Breaking (1.0 contract finalize):** (a) `lanes list --json` renames `steps` (integer count) to `step_count`. The bare key `steps` read like an array but emitted a count, surprising consumers, full step detail lives in `lanes run --dry-run --json`. (b) `lanes import --json` renames `tier` to `trust_tier` to match every other plugin/lane envelope. Last-chance shape break before tagging 1.0 |
| 16 | 2026-04-26 | Add `run_for_pre_release(name, dry_run)`, a silent lane executor used by `release --json --pre-lane <name>` so the release JSON envelope is the only thing on stdout. Subprocess stdout/stderr is redirected to null; failure bails with a plain stderr error and non-zero exit. Pretty-print release path is unchanged and still goes through `LaneAction::Run` |
| 15 | 2026-04-26 | `lanes run --json --dry-run` now emits a `{schema_version: 1, lane, description, total_steps, fail_fast, dry_run: true, steps: [{step, kind, name, items?}]}` envelope. Previously it ignored `--json` and printed prose, breaking the contract that `--json` always means parseable stdout. Per-step `duration_ms` is omitted in dry-run mode (no execution). Behavioral example for the per-step shape on real runs (`steps: [{step, name, success, duration_ms, error}, ...]`) also documented |
| 14 | 2026-04-26 | `lanes import --json` envelope tightened: `file` is now always the computed `.fledge/lanes/<safe_name>.toml` path (string, never null), and a new `written: bool` field signals whether the file was actually created (false when every lane was skipped). Previously `file: null` in the all-skipped case implied the path wasn't computable, but it's a pure function of source, null was misleading |
| 13 | 2026-04-25 | Fix `lanes run --json` prose leak: fledge's own progress prints AND each spawned task's stdout/stderr were going to the agent's stdout, interleaving with the JSON envelope and breaking `--json | jq`. Threaded a `quiet` flag through `execute_task_with_deps` / `execute_inline` / `execute_parallel` / `execute_single_task` / `execute_task_recursive`. In JSON mode prose is suppressed and spawned commands' stdout/stderr are redirected to `/dev/null`. Trade-off: per-step output isn't captured into the JSON record (consumers needing it re-run without `--json`). New regression test `cli_lane_run_json_stdout_is_clean` guards the contract |
| 12 | 2026-04-25 | **Breaking (tier C, #272):** `lanes list --json`, `lanes search --json`, `lanes run --json`, `lanes validate --json` migrated to `{schema_version: 1, <resource>: [...]}` (or flattened with `schema_version` at top for run/validate). `lanes` for list, `results` for search, run/validate get the field added to the existing object. Last-chance shape break before 1.0 |
| 11 | 2026-04-25 | Tier B follow-up: `lanes init`, `import`, `publish`, and `create` honour `--json` and emit `{schema_version:1, action, ...}` envelopes. `init` reports `project_type/lanes_added/file`; `import` reports `source/imported/skipped/file`; `publish` reports `repo/lanes_published/topic/import_hint`; `create` reports `path/name/description/files_created`. Failure paths still exit non-zero, `--json` never silently turns failure into success |
| 10 | 2026-04-25 | Lock parallel-group + `fail_fast` semantics for 1.0: in-flight siblings always finish (no cancellation), `fail_fast` governs only the *next* step. Documents existing behavior, no code change |
| 9 | 2026-04-23 | Add `--json` flag to `lane run` for structured JSON output |
| 8 | 2026-04-22 | Add `create` and `validate` subcommands; `publish` now validates before pushing |
| 7 | 2026-04-21 | Imported lanes stored in `.fledge/lanes/` instead of appending to fledge.toml; lane loading merges both sources |
| 6 | 2026-04-21 | Add step timing, each step prints elapsed time, lane summary includes total |
| 5 | 2026-04-21 | Generalize parallel groups to accept inline commands, not just task refs |
| 4 | 2026-04-21 | Rename from flows to lanes, 1.0 branding |
| 3 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 2 | 2026-04-20 | Add community lane registry (search + import) |
| 1 | 2026-04-20 | Initial spec |
