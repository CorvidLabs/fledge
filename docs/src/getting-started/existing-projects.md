# Using Fledge with Existing Projects

Fledge isn't just for new projects. Most of its features work in any git repo, no setup required.

## Zero-Config: Just Run It

`cd` into any project and fledge auto-detects your stack:

```bash
cd my-existing-project
fledge run test    # detects Rust/Node/Go/Python/Ruby/Java and runs the right command
fledge run build
fledge run lint
```

No `fledge.toml` needed. Fledge looks for marker files (`Cargo.toml`, `package.json`, `go.mod`, etc.) and provides sensible default tasks for Rust, Node.js, Go, Python, Ruby, Java, and Swift. For Node.js projects, it also detects your package manager (npm, bun, yarn, pnpm) from lockfiles.

See the [CLI Reference](../cli-reference.md#fledge-run-task) for the full auto-detection table.

## Lock It In: Generate a Config

When you want to customize tasks, generate a `fledge.toml`:

```bash
fledge run --init
```

This creates a config file pre-filled with the detected tasks. Edit it to add custom commands, change defaults, or define lanes:

```toml
[tasks]
build = "cargo build"
test = "cargo test"
lint = "cargo clippy -- -D warnings"
fmt = "cargo fmt --check"

[lanes.ci]
description = "Full CI pipeline"
steps = ["fmt", "lint", "test", "build"]
```

Once `fledge.toml` exists, it takes full precedence over auto-detection.

## Everything Else Works Too

Every command in the [six pillars](../pillars.md) works in any git repo — AI review, work branches, changelog, doctor, plugins, and more. See the sidebar for the full list.

## Turn Your Project into a Template

Have a project structure you want to reuse? Turn it into a fledge template:

```bash
fledge templates create my-stack
```

This scaffolds a template directory by examining your project. Add Tera variables for the parts that should change (project name, author, etc.), then use it for future projects:

```bash
fledge templates init new-project --template ./my-stack
```

Or publish it for others:

```bash
fledge templates publish ./my-stack
```

## What's Next

Once you're running tasks, the rest of the dev loop is available immediately — see [Quick Start: What's Next](./quick-start.md#whats-next) for the full list.
