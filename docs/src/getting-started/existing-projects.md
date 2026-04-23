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

No `fledge.toml` needed. Fledge looks for marker files (`Cargo.toml`, `package.json`, `go.mod`, etc.) and provides sensible default tasks.

### What Gets Detected

| Project Type | Detected By | Default Tasks |
|-------------|------------|---------------|
| Rust | `Cargo.toml` | build, test, lint, fmt |
| Node.js | `package.json` | test, build, lint, dev (if scripts exist) |
| Go | `go.mod` | build, test, lint |
| Python | `pyproject.toml` / `setup.py` | test, lint, fmt |
| Ruby | `Gemfile` | test, lint |
| Java (Gradle) | `build.gradle` | build, test |
| Java (Maven) | `pom.xml` | build, test |
| Swift | `Package.swift` | build, test |

For Node.js projects, fledge also detects your package manager (npm, bun, yarn, pnpm) from lockfiles and uses the right one.

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

These commands work in any git repo regardless of how the project was created:

| Command | What it does |
|---------|-------------|
| `fledge run` | Task runner (zero-config or from fledge.toml) |
| `fledge lanes` | Workflow pipelines |
| `fledge review` | AI code review of your current branch |
| `fledge ask` | Ask questions about your codebase |
| `fledge work` | Feature branch and PR workflow |
| `fledge checks` | CI/CD status |
| `fledge changelog` | Changelog from git tags |
| `fledge issues` | GitHub issues |
| `fledge prs` | Pull requests |
| `fledge metrics` | Code stats (LOC, churn, test ratio) |
| `fledge deps` | Dependency health |
| `fledge doctor` | Environment diagnostics |

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

## Typical Workflow

1. **Start using fledge today**: `cd your-project && fledge run test`
2. **Optionally lock in config**: `fledge run --init` to generate `fledge.toml`
3. **Set up lanes**: `fledge lanes init` for CI pipelines
4. **Use the full toolkit**: `fledge review`, `fledge work start feature-x`, `fledge checks`
5. **Create templates**: Once you have a setup you like, `fledge templates create` to reuse it
