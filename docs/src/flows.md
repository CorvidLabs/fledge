# Flows & Pipelines

Flows let you chain tasks into named pipelines. Define them in `fledge.toml`, run them with `fledge flow ci`. They support parallel groups and configurable failure behavior.

## Quick Start

Already have tasks in `fledge.toml`? Generate flows automatically:

```bash
fledge flow --init
```

This looks at your project type and creates sensible defaults. Then just run one:

```bash
fledge flow ci
```

## Defining Flows

Flows go in `fledge.toml` alongside your tasks:

```toml
[tasks]
fmt = "cargo fmt --check"
lint = "cargo clippy -- -D warnings"
test = "cargo test"
build = "cargo build"

[flows.ci]
description = "Full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[flows.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
```

### Flow Options

| Field | Type | Default | What it does |
|-------|------|---------|-------------|
| `description` | string | `(no description)` | Shows up when listing flows |
| `steps` | array | required | Ordered list of steps |
| `fail_fast` | bool | `true` | Stop on first failure vs. run everything and report |

## Step Types

You can mix these freely in a flow:

### Task References

Just name a task from your `[tasks]` section. Dependencies (`deps`) get resolved automatically.

```toml
steps = ["lint", "test", "build"]
```

### Inline Commands

One-off shell commands without cluttering your task list:

```toml
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "echo 'Build complete'" },
]
```

### Parallel Groups

Run multiple tasks at the same time. Everything in the group finishes before the next step starts.

```toml
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build"
]
```

Here `fmt` and `lint` run concurrently, then `test`, then `build`.

## Failure Behavior

Default is `fail_fast = true`. Pipeline stops on the first failure.

```toml
[flows.ci]
description = "Stop on first failure"
steps = ["lint", "test", "build"]
```

Set `fail_fast = false` when you want the full picture:

```toml
[flows.audit]
description = "Run everything, report all failures"
fail_fast = false
steps = ["lint", "test", "security-check", "license-check"]
```

## Task Configuration

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

## Examples

### CI Pipeline

```toml
[flows.ci]
description = "Full CI pipeline"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build"
]
```

### Release

```toml
[flows.release]
description = "Build and package a release"
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "strip target/release/my-app" },
  { run = "tar -czf release.tar.gz -C target/release my-app" },
]
```

### Full Audit

```toml
[flows.audit]
description = "All quality checks"
fail_fast = false
steps = [
  "lint",
  "test",
  { run = "cargo audit" },
  { run = "cargo deny check" },
]
```

## Auto-Generated Defaults

`fledge flow --init` detects your project type:

| Project | How it's detected | What you get |
|---------|------------------|-------------|
| Rust | `Cargo.toml` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Node.js | `package.json` | `ci` (lint, test, build), `check` (parallel lint+test) |
| Go | `go.mod` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Python | `pyproject.toml` | `ci` (fmt, lint, typecheck, test), `check` (parallel fmt+lint, test) |

## CLI

```bash
fledge flow              # list flows (same as fledge flow list)
fledge flow run ci       # run one
fledge flow run ci --dry-run # preview the plan
fledge flow init         # generate defaults
fledge flow list --json
fledge flow search              # find community flows
fledge flow search rust         # search with keyword
fledge flow import owner/repo   # import flows from GitHub
fledge flow import owner/repo@v1.0.0  # pin to a version
```

## Community Flow Registry

Share and discover flows via GitHub. Repos with the `fledge-flow` topic are discoverable through `fledge flow search`.

### Official Examples

[CorvidLabs/fledge-flows](https://github.com/CorvidLabs/fledge-flows) is the official collection of language-specific flow examples. Each subdirectory contains a fully-documented `fledge.toml`.

| Language | Import command |
|----------|---------------|
| Rust | `fledge flow import CorvidLabs/fledge-flows/rust` |
| Python | `fledge flow import CorvidLabs/fledge-flows/python` |
| Node/TypeScript | `fledge flow import CorvidLabs/fledge-flows/node-typescript` |
| Go | `fledge flow import CorvidLabs/fledge-flows/go` |

### Publishing Flows

1. Create a repo with a `fledge.toml` containing your flows and tasks
2. Add the `fledge-flow` topic to the repo on GitHub
3. Others can find it with `fledge flow search` and import it

### Importing Flows

```bash
fledge flow import CorvidLabs/fledge-flows
```

This fetches the remote repo's `fledge.toml`, extracts its flows and any required tasks, and merges them into your local `fledge.toml`. Existing flows with the same name are skipped (not overwritten).

You can pin to a specific branch or tag:

```bash
fledge flow import CorvidLabs/fledge-flows@v1.0.0
```

## Tips

- Start with `fledge flow init` and customize from there.
- Use parallel groups for independent checks. Linting and formatting don't need to wait for each other.
- Keep `fail_fast = true` for CI. No point building if tests fail.
- Use `fail_fast = false` for audit flows where you want the full report.
- Inline commands are great for one-off steps that don't need to be named tasks.
