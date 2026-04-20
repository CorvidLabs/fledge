# Lanes & Pipelines

Lanes are composable workflow pipelines defined in `fledge.toml`. They chain multiple tasks into named sequences with support for parallel execution and configurable failure behavior.

## Quick Start

If you already have tasks in `fledge.toml`, add default lanes automatically:

```bash
fledge lane --init
```

This detects your project type and generates appropriate lane definitions. Then run one:

```bash
fledge lane ci
```

## Defining Lanes

Lanes live in `fledge.toml` alongside your tasks. Each lane has a name, optional description, a list of steps, and an optional `fail_fast` flag.

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

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `description` | string | `(no description)` | Shown when listing lanes |
| `steps` | array | required | Ordered list of steps to execute |
| `fail_fast` | bool | `true` | Stop on first failure. Set to `false` to run all steps and report failures at the end |

## Step Types

Lanes support three types of steps that can be mixed freely:

### Task References

Reference any task defined in the `[tasks]` section by name. Task dependencies (`deps`) are resolved automatically.

```toml
steps = ["lint", "test", "build"]
```

### Inline Commands

Run a shell command directly without defining it as a named task. Useful for one-off steps.

```toml
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "echo 'Build complete'" },
]
```

### Parallel Groups

Run multiple tasks concurrently. All tasks in the group must complete before the next step begins.

```toml
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build"
]
```

In this example, `fmt` and `lint` run at the same time. Once both finish, `test` runs, then `build`.

## Failure Behavior

By default, lanes use `fail_fast = true` — the pipeline stops immediately when any step fails.

```toml
[lanes.ci]
description = "Stop on first failure"
steps = ["lint", "test", "build"]
# fail_fast = true (default)
```

Set `fail_fast = false` to run all steps regardless of failures, then report a summary:

```toml
[lanes.audit]
description = "Run all checks, report all failures"
fail_fast = false
steps = ["lint", "test", "security-check", "license-check"]
```

## Task Configuration

Tasks referenced by lanes support the full task configuration format:

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

| Field | Type | Description |
|-------|------|-------------|
| `cmd` | string | Shell command to execute |
| `description` | string | Shown when listing tasks |
| `deps` | array | Tasks to run first (resolved recursively) |
| `env` | table | Environment variables for this task |
| `dir` | string | Working directory (relative to project root) |

When a lane step references a task with `deps`, those dependencies run first automatically.

## Real-World Examples

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

### Release Workflow

```toml
[lanes.release]
description = "Build and prepare release"
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "strip target/release/my-app" },
  { run = "tar -czf release.tar.gz -C target/release my-app" },
]
```

### Full Audit (No Fail-Fast)

```toml
[lanes.audit]
description = "Run all quality checks"
fail_fast = false
steps = [
  "lint",
  "test",
  { run = "cargo audit" },
  { run = "cargo deny check" },
]
```

## Auto-Generated Defaults

`fledge lane --init` generates language-aware defaults:

| Project Type | Detection | Default Lanes |
|--------------|-----------|---------------|
| Rust | `Cargo.toml` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Node.js | `package.json` | `ci` (lint, test, build), `check` (parallel lint+test) |
| Go | `go.mod` | `ci` (fmt, lint, test, build), `check` (parallel fmt+lint, test) |
| Python | `pyproject.toml` | `ci` (fmt, lint, typecheck, test), `check` (parallel fmt+lint, test) |

## CLI Usage

```bash
# List available lanes
fledge lane

# Run a lane
fledge lane ci

# Preview without running
fledge lane ci --dry-run

# Add default lanes to fledge.toml
fledge lane --init

# JSON output (for scripting)
fledge lane --list --json
```

## Tips

- Start with `fledge lane --init` to get sensible defaults, then customize.
- Use parallel groups for independent checks (linting and formatting don't depend on each other).
- Keep `fail_fast = true` (the default) for CI — there's no point running the build if tests fail.
- Use `fail_fast = false` for audit-style lanes where you want a complete report.
- Combine inline commands with task references for flexibility without polluting your task list.
