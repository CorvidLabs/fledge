# Lanes & Pipelines

Lanes let you chain tasks into named pipelines. Define them in `fledge.toml`, run them with `fledge lane ci`. They support parallel groups and configurable failure behavior.

## Quick Start

Already have tasks in `fledge.toml`? Generate lanes automatically:

```bash
fledge lane --init
```

This looks at your project type and creates sensible defaults. Then just run one:

```bash
fledge lane ci
```

## Defining Lanes

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

### Lane Options

| Field | Type | Default | What it does |
|-------|------|---------|-------------|
| `description` | string | `(no description)` | Shows up when listing lanes |
| `steps` | array | required | Ordered list of steps |
| `fail_fast` | bool | `true` | Stop on first failure vs. run everything and report |

## Step Types

You can mix these freely in a lane:

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

Default is `fail_fast = true` — pipeline stops as soon as something fails.

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
[lanes.ci]
description = "Full CI pipeline"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build"
]
```

### Release

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

### Full Audit

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

## Auto-Generated Defaults

`fledge lane --init` detects your project type:

| Project | How it's detected | What you get |
|---------|------------------|-------------|
| Rust | `Cargo.toml` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Node.js | `package.json` | `ci` (lint, test, build), `check` (parallel lint+test) |
| Go | `go.mod` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Python | `pyproject.toml` | `ci` (fmt, lint, typecheck, test), `check` (parallel fmt+lint, test) |

## CLI

```bash
fledge lane              # list lanes
fledge lane ci           # run one
fledge lane ci --dry-run # preview the plan
fledge lane --init       # generate defaults
fledge lane --list --json
```

## Tips

- Start with `fledge lane --init` and customize from there.
- Use parallel groups for independent checks — linting and formatting don't need to wait for each other.
- Keep `fail_fast = true` for CI. No point building if tests fail.
- Use `fail_fast = false` for audit lanes where you want the full report.
- Inline commands are great for one-off steps that don't need to be named tasks.
