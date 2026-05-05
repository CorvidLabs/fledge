# Run: Tasks and Lanes

The Run pillar covers three commands: `fledge run` (task runner), `fledge watch` (file watcher), and `fledge lanes` (pipelines).

## Running Tasks

`fledge run` works immediately in any project. No config needed. It detects your stack from marker files and provides sensible defaults:

```bash
fledge run test     # auto-detects Rust/Node/Go/Python/Ruby/Java/Swift
fledge run build
fledge run lint
fledge run --list   # see what's available
```

### Auto-Detection

| Project | Detected by | Default tasks |
|---------|------------|---------------|
| Rust | `Cargo.toml` | build, test, lint, fmt |
| Node.js | `package.json` | test, build, lint, dev (if scripts exist) |
| Go | `go.mod` | build, test, lint |
| Python | `pyproject.toml` / `setup.py` | test, lint, fmt |
| Ruby | `Gemfile` | test, lint |
| Swift | `Package.swift` | build, test |
| Gradle | `build.gradle` | build, test |
| Maven | `pom.xml` | build, test |

For Node.js projects, fledge also detects your package manager (npm, bun, yarn, pnpm) from lockfiles.

### Config Mode

When you want full control, generate a `fledge.toml`:

```bash
fledge run --init
```

This creates a config file pre-filled with detected tasks. Once `fledge.toml` exists, it takes full precedence. No mixing with auto-detection. You can also override the detected language with `--lang`:

```bash
fledge run test --lang swift
```

## Watching for Changes

`fledge watch` re-runs a task automatically when files change:

```bash
fledge watch test            # re-run tests on save
fledge watch build           # rebuild on change
```

It watches the project directory, ignoring `.git/`, `target/`, `node_modules/`, and other common build directories. The debounce interval defaults to 500ms and can be changed with `--debounce <MS>`.

## Lanes

Lanes let you chain tasks into named pipelines. Define them in `fledge.toml`, run them with `fledge lanes run ci`. They support parallel groups and configurable failure behavior.

### Quick Start

Already have tasks in `fledge.toml`? Generate lanes automatically:

```bash
fledge lanes init
```

This looks at your project type and creates sensible defaults. Then just run one:

```bash
fledge lanes run ci
```

### Defining Lanes

Lanes go in `fledge.toml` alongside your tasks:

```toml
[tasks]
fmt = "cargo fmt --check"
lint = "cargo clippy -- -D warnings"
test = "cargo test"
build = "cargo build"

[lanes.ci]
description = "Full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
```

#### Lane Options

| Field | Type | Default | What it does |
|-------|------|---------|-------------|
| `description` | string | `(no description)` | Shows up when listing lanes |
| `steps` | array | required | Ordered list of steps |
| `fail_fast` | bool | `true` | Stop on first failure vs. run everything and report |

### Step Types

You can mix these freely in a lane:

#### Task References

Just name a task from your `[tasks]` section. Dependencies (`deps`) get resolved automatically.

```toml
steps = ["lint", "test", "build"]
```

#### Inline Commands

One-off shell commands without cluttering your task list:

```toml
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "echo 'Build complete'" },
]
```

#### Parallel Groups

Run multiple items at the same time. Everything in the group finishes before the next step starts. Items can be task references or inline commands.

```toml
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build"
]
```

Here `fmt` and `lint` run concurrently, then `test`, then `build`.

You can mix task references and inline commands in a parallel group:

```toml
steps = [
  { parallel = ["lint", { run = "echo checking..." }, "fmt"] },
  "test"
]
```

### Step Options

Table-form steps accept four optional fields: `when`, `timeout`, `retries`, and `retry_delay`. They work on task references (`{ task = "name" }`), inline commands (`{ run = "..." }`), and parallel groups (`{ parallel = [...] }`).

| Option | Type | Default | What it does |
|--------|------|---------|--------------|
| `when` | string | always run | Skip the step unless an env-var condition is met. See forms below. |
| `timeout` | integer (seconds) | unlimited | Per-attempt deadline. The whole process tree is killed on exceed. |
| `retries` | integer | `0` | Retry attempts after failure. Total attempts = `retries + 1`. |
| `retry_delay` | integer (seconds) | `1` | Sleep between retry attempts. Set `0` for immediate retry. |

```toml
[lanes.release]
description = "Build and ship"
steps = [
  { task = "test", when = "!SKIP_TESTS" },
  { task = "build", timeout = 120 },
  { run = "scripts/publish.sh", retries = 3, retry_delay = 5 },
  { task = "deploy", when = "CI=true,DEPLOY_ENV=production", timeout = 60 },
]
```

#### Conditional steps with `when`

The `when` string supports four condition forms. Multiple comma-separated conditions are AND'd.

| Form | Meaning |
|------|---------|
| `VAR` | Run when `VAR` is set and non-empty |
| `VAR=value` | Run when `VAR` equals `value` exactly |
| `!VAR` | Run when `VAR` is unset or empty |
| `!VAR=value` | Run when `VAR` does not equal `value` |
| `VAR1,VAR2=x` | AND: every condition must hold |

Skipped steps are visible in the output (`⏭ Step N name (skipped: when 'X' not met)`) and in JSON output (`"skipped": true, "reason": "..."`).

#### Per-step timeouts

`timeout` sets a per-attempt deadline. On exceed, fledge sends `SIGKILL` to the process group on Unix or `TerminateJobObject` on Windows, so multi-statement shells (`sh -c "a && b"`, `cmd /c "a & b"`) don't leak grandchildren. If the step has `retries`, each attempt gets a fresh deadline.

#### Retries with `retry_delay`

`retries` re-runs the entire step on failure. `retry_delay` controls the sleep between attempts (default 1s). `retry_delay = 0` retries immediately — useful when you want to absorb a flake without slowing down the lane.

```toml
# Immediate-retry flake mitigation
{ run = "curl https://api.example.com/health", retries = 5, retry_delay = 0 }
```

### Resuming with `--from`

`fledge lanes run <name> --from <step>` skips every step before the target. Useful for "I already ran lint and test, just resume from build."

```bash
fledge lanes run ci --from build      # by step name
fledge lanes run ci --from 3          # by 1-based index
```

Resume is stateless — no run history is persisted. Skipped steps appear in the output (`⏭ Step N (skipped by --from)`) and in JSON output (`"skipped": true, "reason": "--from"`). Targeting a parallel-group step by name doesn't work; use the index instead.

### Failure Behavior

Default is `fail_fast = true`. Pipeline stops on the first failure.

```toml
[lanes.ci]
description = "Stop on first failure"
steps = ["lint", "test", "build"]
```

Set `fail_fast = false` when you want the full picture:

```toml
[lanes.audit]
description = "Run everything, report all failures"
fail_fast = false
steps = ["lint", "test", "security-check", "license-check"]
```

### Step Timing

Every step prints its elapsed time, and the lane summary shows total time:

```text
▶️ Lane: ci, Full CI pipeline
  ▶️ Running parallel: fmt, lint
  ✔ Step 1 done (245ms)
  ▶️ Running task: test
  ✔ Step 2 done (1.032s)
  ▶️ Running task: build
  ✔ Step 3 done (3.456s)
✅ Lane ci completed (3 steps in 4.733s)
```

This helps identify slow steps in your pipeline without any extra tooling.

## Task Configuration

Tasks are the building blocks that lanes and `fledge run` execute. Define them in `fledge.toml`.

### Short Form

```toml
[tasks]
lint = "cargo clippy"
```

### Full Form

```toml
[tasks.build]
cmd = "cargo build --release"
description = "Build release binary"
deps = ["lint"]
env = { RUST_LOG = "info" }
dir = "crates/core"
```

| Field | Type | What it does |
|-------|------|-------------|
| `cmd` | string | Shell command to run |
| `description` | string | Shows up when listing tasks |
| `deps` | array | Tasks to run first (resolved recursively) |
| `env` | table | Environment variables for this task |
| `dir` | string | Working directory (relative to project root) |

## Lane Examples

#### CI Pipeline

```toml
[lanes.ci]
description = "Full CI pipeline"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build"
]
```

#### Release

```toml
[lanes.release]
description = "Build and package a release"
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "strip target/release/my-app" },
  { run = "tar -czf release.tar.gz -C target/release my-app" },
]
```

#### Full Audit

```toml
[lanes.audit]
description = "All quality checks"
fail_fast = false
steps = [
  "lint",
  "test",
  { run = "cargo audit" },
  { run = "cargo deny check" },
]
```

#### Real CI with conditional deploy and flake retries

```toml
[lanes.ci]
description = "Lint, test, build, and deploy on main"
steps = [
  { parallel = ["fmt", "lint"] },
  { task = "test", timeout = 300 },
  "build",
  { task = "deploy", when = "CI=true,BRANCH=main", timeout = 120, retries = 2, retry_delay = 5 },
]
```

### Auto-Generated Defaults

`fledge lanes init` detects your project type:

| Project | How it's detected | What you get |
|---------|------------------|-------------|
| Rust | `Cargo.toml` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Node.js | `package.json` | `ci` (lint, test, build), `check` (parallel lint+test) |
| Go | `go.mod` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Python | `pyproject.toml` | `ci` (fmt, lint, test), `check` (parallel fmt+lint, test) |

## Lanes CLI

```bash
fledge lanes run ci                   # run a lane
fledge lanes run ci --dry-run         # preview the plan
fledge lanes run ci --from build      # resume from a step (by name or 1-based index)
fledge lanes run ci --json            # machine-readable per-step results
fledge lanes list                     # list lanes
fledge lanes list --json
fledge lanes init                     # generate defaults
fledge lanes search                   # find community lanes
fledge lanes search rust              # search with keyword
fledge lanes import owner/repo        # import lanes from GitHub
fledge lanes import owner/repo@v1.0.0 # pin to a version
fledge lanes publish --org MyOrg      # publish lanes to GitHub
fledge lanes create my-lanes          # scaffold a new lane repo
fledge lanes validate                 # validate lane definitions
fledge lanes validate --strict        # treat warnings as errors
fledge lanes validate --json          # machine-readable output
```

### Community Lane Registry

Share and discover lanes via GitHub. Repos with the `fledge-lane` topic are discoverable through `fledge lanes search`.

#### Official Examples

[CorvidLabs/fledge-lanes](https://github.com/CorvidLabs/fledge-lanes) is the official collection of language-specific lane examples. Each subdirectory contains a fully-documented `fledge.toml`.

| Language | Import command |
|----------|---------------|
| Rust | `fledge lanes import CorvidLabs/fledge-lanes/rust` |
| Python | `fledge lanes import CorvidLabs/fledge-lanes/python` |
| Node/TypeScript | `fledge lanes import CorvidLabs/fledge-lanes/node-typescript` |
| Go | `fledge lanes import CorvidLabs/fledge-lanes/go` |

#### Creating Lanes

Use `fledge lanes create` to scaffold a ready-to-publish lane repo:

```bash
fledge lanes create my-lanes
```

This creates a directory with a starter `fledge.toml` containing example tasks and lanes, a README, and a `.gitignore`. Edit the lanes, then validate and publish:

```bash
fledge lanes validate ./my-lanes     # check for errors
fledge lanes publish ./my-lanes      # push to GitHub (validates first)
```

#### Publishing Lanes

1. Create a repo with a `fledge.toml` containing your lanes and tasks (or use `fledge lanes create`)
2. Validate with `fledge lanes validate` (publish does this automatically)
3. Publish with `fledge lanes publish` (sets the `fledge-lane` topic automatically)
4. Others can find it with `fledge lanes search` and import it

#### Importing Lanes

```bash
fledge lanes import CorvidLabs/fledge-lanes
```

This fetches the remote repo's `fledge.toml`, extracts its lanes and any required tasks, and merges them into your local `fledge.toml`. Existing lanes with the same name are skipped (not overwritten).

You can pin to a specific branch or tag:

```bash
fledge lanes import CorvidLabs/fledge-lanes@v1.0.0
```

### Related

- [`fledge.toml` Reference](./fledge-toml.md). Full schema for tasks, lanes, release, and imported lanes
- [Configuration](./configuration.md). Global config, GitHub tokens
- [Extend: Plugins](./plugins.md). Community commands, use plugins in lanes
- [CLI Reference](./cli-reference.md). Full `fledge lanes` subcommand reference
- [Example Lanes](https://github.com/CorvidLabs/fledge-lanes). Official community lane collection

### Tips

- Start with `fledge lanes init` and customize from there.
- Use parallel groups for independent checks. Linting and formatting don't need to wait for each other.
- Keep `fail_fast = true` for CI. No point building if tests fail.
- Use `fail_fast = false` for audit lanes where you want the full report.
- Inline commands are great for one-off steps that don't need to be named tasks.
