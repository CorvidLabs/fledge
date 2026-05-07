# `fledge.toml` Reference

`fledge.toml` lives in your project root and defines tasks, lanes, and release behavior. It's read by `fledge run`, `fledge lanes`, `fledge release`, and `fledge watch`. Plugins with the `metadata` capability can read it through the `fledge_config` metadata key.

If no `fledge.toml` exists, `fledge run` falls back to language-aware auto-detection. As soon as the file exists, it takes full precedence. There is no merging with auto-detection.

For plugin manifests (`plugin.toml`), see [Extend: Plugins](./plugins.md). For global user config (`~/.config/fledge/config.toml`), see [Configuration](./configuration.md).

## Creating it

```bash
fledge run --init                  # starter file with detected tasks
fledge lanes init                  # adds default lanes for your stack
fledge templates init my-app       # included in scaffolded projects
```

## File layout at a glance

```toml
schema_version = 1                  # optional, currently unused

[tasks]                             # individual commands you can run
build = "cargo build"

[tasks.test]                        # full form
cmd = "cargo test"
description = "Run the test suite"
deps = ["build"]
env = { RUST_LOG = "debug" }
dir = "."

[lanes.ci]                          # named pipelines that chain tasks
description = "Full CI pipeline"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build",
]
fail_fast = true

[release]                           # extra files to bump on `fledge release`
files = ["flake.nix"]
```

That's the entire schema. Everything else on this page expands one of these sections.

## `[tasks]`

Tasks are the building blocks. Lanes reference them by name, and you run them directly with `fledge run <name>`.

### Short form

A bare command string:

```toml
[tasks]
build = "cargo build"
test = "cargo test"
lint = "cargo clippy --all-targets -- -D warnings"
```

Short-form tasks have no description, no deps, no env, no working directory.

### Full form

A table when you need more than a command:

```toml
[tasks.deploy]
cmd = "cargo install --path ."
description = "Build and install locally"
deps = ["test"]
env = { RUST_LOG = "info" }
dir = "crates/cli"
```

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `cmd` | string | yes | n/a | Shell command. Run via `sh -c` (Unix) or `cmd /C` (Windows). |
| `description` | string | no | (uses `cmd`) | Shown by `fledge run --list`. |
| `deps` | array of strings | no | `[]` | Tasks to run first. Resolved recursively. Cycles are detected and rejected. |
| `env` | table | no | `{}` | Environment variables set for the task's process. |
| `dir` | string | no | project root | Working directory, relative to the project root. |

### Mixing forms

You can mix short and full in the same file:

```toml
[tasks]
build = "cargo build"
fmt = "cargo fmt"

[tasks.ci]
cmd = "cargo test --workspace"
description = "Full workspace test"
deps = ["fmt", "build"]
```

## `[lanes]`

Lanes chain tasks (and inline commands) into named pipelines. Run with `fledge lanes run <name>`.

```toml
[lanes.ci]
description = "Full CI pipeline"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build",
]
fail_fast = true
```

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `description` | string | no | `"(no description)"` | Shown by `fledge lanes list`. |
| `steps` | array | yes | n/a | Ordered steps. At least one is required. |
| `fail_fast` | bool | no | `true` | If `false`, every step runs even after failures and a summary is reported at the end. |

### Step types

A step can be one of three shapes. Mix them freely. Bare-string task references are shorthand; the table form (`{ task = "name" }`) is required when you want step options like `when`, `timeout`, `retries`, or `retry_delay`.

| Shape | Form | Notes |
|-------|------|-------|
| Task reference (short) | `"name"` | Bare string. Shorthand for `{ task = "name" }`. |
| Task reference (full) | `{ task = "name" }` | Table form. Accepts step options. |
| Inline command | `{ run = "..." }` | One-off shell command. Accepts step options. |
| Parallel group | `{ parallel = [...] }` | Items run concurrently. Accepts step options. |

#### Task reference

Just a string naming a task in `[tasks]`:

```toml
steps = ["lint", "test", "build"]
```

Task `deps` resolve automatically before the step runs.

#### Inline command

A one-off shell command without cluttering `[tasks]`:

```toml
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "tar -czf release.tar.gz -C target/release my-app" },
]
```

#### Parallel group

Items inside a `parallel` array run concurrently. The lane waits for all of them before the next step:

```toml
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
]
```

Items in a parallel group can be task references or inline commands:

```toml
steps = [
  { parallel = [
      "lint",
      { run = "cargo audit" },
      "fmt",
  ] },
  "test",
]
```

Parallel groups cannot be nested.

### Step options

Table-form steps (`{ task = "..." }`, `{ run = "..." }`, `{ parallel = [...] }`) accept four optional fields.

| Option | Type | Default | Notes |
|--------|------|---------|-------|
| `when` | string | always run | Skip the step unless an env-var condition is met. Forms: `VAR` (set & non-empty), `VAR=value` (equals), `!VAR` (unset/empty), `!VAR=value` (not equals). Comma-separated values are AND'd. |
| `timeout` | integer | unlimited | Per-attempt deadline in seconds. The whole process tree is killed on exceed (Unix: `killpg(SIGKILL)`; Windows: `TerminateJobObject`). Includes task-dependency resolution. |
| `retries` | integer | `0` | Retry attempts after failure. Total attempts = `retries + 1`. The step re-runs as a whole; per-step, not per-command. |
| `retry_delay` | integer | `1` | Sleep between retry attempts in seconds. Set `0` for immediate retry. Only meaningful when `retries > 0`. |

```toml
[lanes.release]
description = "Test, build, deploy with retries"
steps = [
  { task = "test", when = "!SKIP_TESTS", timeout = 300 },
  { task = "build", timeout = 600 },
  { run = "scripts/publish.sh", retries = 3, retry_delay = 5 },
  { task = "deploy", when = "CI=true,BRANCH=main", timeout = 120 },
]
```

Skipped steps appear in the human-readable output (`⏭ Step N <label> (skipped: when 'X' not met)`) and in `--json` output (`"skipped": true, "reason": "..."`). The `--from` flag adds its own skip reason: `"reason": "--from"`.

### `fail_fast`

```toml
[lanes.audit]
description = "Run everything, report all failures"
fail_fast = false
steps = ["lint", "test", "audit"]
```

| Value | Behavior |
|-------|----------|
| `true` (default) | Stop at the first failed step. |
| `false` | Run every step; print a final report listing every failure. |

Use `fail_fast = false` for "give me the full picture" lanes (audit, broad checks). Keep `true` for CI gates where there's no point continuing after a break.

## `[release]`

Controls additional files that `fledge release` bumps alongside the language's standard manifest (`Cargo.toml`, `package.json`, `pyproject.toml`, `pom.xml`, `build.gradle`, `setup.cfg`, or `plugin.toml`).

```toml
[release]
files = ["flake.nix", "docs/install.md"]
```

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `files` | array of strings | `[]` | Extra files whose `version "X.Y.Z"` line should be rewritten on release. Paths are relative to the project root and may not escape it. |

The bumper looks for the regex `version\s*[=:]\s*["']?(\d+\.\d+\.\d+)` and rewrites the matched version. Files without a matching line are silently skipped.

`fledge release --dry-run` reports the same set of files the real run would bump, including these extras.

> **Heads-up for plugin authors:** Don't list Homebrew formulae here. Formulae need a fresh `sha256` per release that doesn't exist until release artifacts are uploaded; bump them in a follow-up workflow instead (see `post-release-formula.yml`).

## `schema_version`

```toml
schema_version = 1
```

Currently parsed but unused. Reserved for future breaking-change migrations. Safe to omit.

## Imported lanes (`.fledge/lanes/*.toml`)

`fledge lanes import <source>` writes imported lane definitions to a generated file under `.fledge/lanes/`. The filename is derived from the source ref as `<owner>-<repo>[-<subpath>].toml` (lowercased, with `/` in subpaths replaced by `-`). For example, `fledge lanes import CorvidLabs/fledge-lanes/rust` writes to `.fledge/lanes/corvidlabs-fledge-lanes-rust.toml`. These files are auto-loaded by `fledge lanes run` after your top-level `fledge.toml`. Your local definitions win on name collisions.

Each imported file has the same shape as `fledge.toml` but typically contains only `[tasks]` and `[lanes]` from the upstream source:

```toml
# Imported from CorvidLabs/fledge-lanes/rust@v1.0.0

[tasks]
fmt-check = "cargo fmt --check"

[lanes.rust-ci]
description = "Rust CI from CorvidLabs/fledge-lanes"
steps = ["fmt-check", "lint", "test", "build"]
```

The leading `# Imported from <source>` comment is parsed and used to display the lane's trust tier in `fledge lanes list`.

## Full worked example

A real-world Rust project mixing every section:

```toml
# fledge.toml. example project

schema_version = 1

[tasks]
build       = "cargo build"
test        = "cargo test"
lint        = "cargo clippy --all-targets -- -D warnings"
fmt         = "cargo fmt --check"
fmt-fix     = "cargo fmt"
audit       = "cargo audit"

[tasks.docs-serve]
cmd = "mdbook serve docs --open"
description = "Serve docs locally with live reload"

[tasks.deploy]
cmd = "scripts/deploy.sh"
description = "Deploy the API"
deps = ["test", "build"]
env = { DEPLOY_TARGET = "staging" }
dir = "."

[lanes.check]
description = "Quick pre-commit check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
]

[lanes.ci]
description = "Full CI pipeline"
steps = [
  { parallel = ["fmt", "lint"] },
  "test",
  "build",
]

[lanes.audit]
description = "Every quality check, report all failures"
fail_fast = false
steps = ["fmt", "lint", "test", "audit"]

[lanes.release]
description = "Pre-release validation"
steps = ["fmt", "lint", "test", "build", "audit"]

[release]
files = ["flake.nix"]
```

## Validating

| Command | What it checks |
|---------|---------------|
| `fledge lanes validate` | Lane references resolve, no dependency cycles, lane structure is well-formed. |
| `fledge lanes validate --strict` | Treats warnings as errors. Useful in CI. |
| `fledge lanes validate --json` | Machine-readable report. |
| `fledge run --list` | Parses `[tasks]` and shows everything fledge can see. |
| `fledge lanes list` | Parses `[lanes]` and shows every lane plus its source (local vs. imported). |

A parse failure on any section bails with a context-rich error pointing at the offending TOML.

## Behavior reference

A few subtle but documented behaviors worth knowing:

- **Project precedence.** If `fledge.toml` exists, auto-detection is fully disabled. Fledge uses only what's in the file.
- **Empty `[tasks]` is an error.** `fledge run` bails with a "no tasks defined" message rather than silently running nothing.
- **Empty `[lanes]` is an error for `fledge lanes run`.** The error tells you to add lanes, import them, or run `fledge lanes init`.
- **Unknown fields are ignored.** TOML keys not in the schema are silently accepted (forward-compatible) but have no effect.
- **`description` is optional everywhere.** Tasks fall back to showing `cmd`, lanes fall back to `(no description)`.
- **Path handling differs by field.** Task `dir` is joined to the project root as configured and is *not* currently canonicalized. Values like `"../sibling"` are accepted and will resolve outside the project. `[release].files` paths *are* canonicalized and rejected if they escape the project root.

## Related

- [Run: Tasks and Lanes](./lanes.md). Workflow walkthrough, auto-detection, watch mode
- [Configuration](./configuration.md). Global `~/.config/fledge/config.toml`
- [Extend: Plugins](./plugins.md). `plugin.toml` reference and ecosystem
- [Plugin Protocol Spec](https://github.com/CorvidLabs/fledge/blob/main/specs/plugin/plugin-protocol.spec.md). `fledge-v1` JSON wire protocol
- [CLI Reference](./cli-reference.md). Every subcommand and flag
