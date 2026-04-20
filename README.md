# fledge

[![CI](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml/badge.svg)](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/fledge)](https://crates.io/crates/fledge)
[![Downloads](https://img.shields.io/crates/d/fledge)](https://crates.io/crates/fledge)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-brightgreen)](https://corvidlabs.github.io/fledge/)

One CLI, six stages, your whole dev lifecycle.

I got tired of juggling `cookiecutter` for scaffolding, `make` for tasks, `gh` for GitHub stuff, and a dozen scripts to glue it all together. So I built fledge — a single Rust binary that handles the full loop from `init` to `changelog`.

## Why fledge?

- **It's fast.** Native Rust binary. No runtime, no node_modules, no waiting around.
- **Smart defaults.** Pulls your name and org from git config, auto-detects your project type, generates sensible task configs.
- **Remote templates.** Any GitHub repo works as a template with `owner/repo` syntax. No special registry needed.
- **Six pillars.** Start, Build, Develop, Review, Ship, Extend — every stage of your project has a home.
- **Lanes.** Chain tasks into pipelines with parallel groups. `fledge lane ci` and you're done.
- **Plugins.** Git-style subcommand pattern. Drop in community extensions or write your own.
- **Language-agnostic.** Auto-detects Rust, Node, Go, Python, Ruby, Java, Swift and adapts.
- **Safe.** Remote template hooks always ask before running. No surprises.
- **Optional TUI.** Interactive template browser if you want it (`--features tui`).

## Install

```bash
# From crates.io (easiest)
cargo install fledge

# With TUI support
cargo install fledge --features tui

# Homebrew
brew install CorvidLabs/tap/fledge

# Install script
curl -fsSL https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.sh | sh

# Nix
nix run github:CorvidLabs/fledge

# From source
git clone https://github.com/CorvidLabs/fledge.git
cd fledge && cargo install --path .
```

## Quick Start

```bash
# Scaffold a Rust CLI
fledge init my-tool --template rust-cli

# Don't know what you want? Browse interactively
fledge init my-project

# Use a template from GitHub
fledge init my-app --template CorvidLabs/fledge-templates/react-app

# See what you'd get before committing
fledge init my-tool --template rust-cli --dry-run

# Set up tasks and run them
fledge run --init       # auto-generates fledge.toml
fledge run build
fledge run test

# Workflow pipelines
fledge lane --init       # generate default lanes
fledge lane ci           # run lint + test + build

# Project health
fledge doctor            # check your environment
fledge metrics           # LOC by language
fledge deps --outdated   # stale dependencies

# Plugins
fledge plugin search deploy
fledge plugin install someone/fledge-deploy

# CI + changelogs
fledge checks
fledge changelog
```

## Built-in Templates

| Template | What you get |
|----------|--------------|
| `rust-cli` | Rust CLI with clap, CI, release automation |
| `ts-bun` | TypeScript project on Bun |

These ship offline with the binary. For more templates (Angular, Go, Python, Swift, monorepo, etc.), see [CorvidLabs/fledge-templates](https://github.com/CorvidLabs/fledge-templates) — 12 official templates and growing.

## CLI Reference

Full docs at [corvidlabs.github.io/fledge](https://corvidlabs.github.io/fledge/). Here's the quick version:

### Start — Scaffold and discover

| Command | What it does |
|---------|-------------|
| `fledge init <name>` | Create a project from a template |
| `fledge list` | Show available templates |
| `fledge search [query]` | Find templates on GitHub |
| `fledge create-template <name>` | Scaffold a new template |
| `fledge publish [path]` | Push a template to GitHub |
| `fledge validate-template [path]` | Check a template for issues |
| `fledge update` | Re-apply source template to existing project |

### Build — Configure and run

| Command | What it does |
|---------|-------------|
| `fledge run [task]` | Run tasks from fledge.toml |
| `fledge lane [name]` | Run a workflow pipeline |
| `fledge config <action>` | Manage global config |
| `fledge doctor` | Environment diagnostics |

### Develop — Branch and spec

| Command | What it does |
|---------|-------------|
| `fledge work <action>` | Feature branches + PRs |
| `fledge spec <action>` | Spec-sync management |

### Review — Quality and insight

| Command | What it does |
|---------|-------------|
| `fledge review` | AI code review via Claude |
| `fledge ask <question>` | Ask about your codebase |
| `fledge metrics` | Code metrics (LOC, churn, test ratio) |
| `fledge deps` | Dependency health (outdated, audit, licenses) |

### Ship — Track and release

| Command | What it does |
|---------|-------------|
| `fledge issues` | List/view GitHub issues |
| `fledge prs` | List/view pull requests |
| `fledge checks` | CI/CD status |
| `fledge changelog` | Generate changelog from git tags |

### Extend — Grow the tool

| Command | What it does |
|---------|-------------|
| `fledge plugin <action>` | Install, remove, search, run plugins |
| `fledge completions [shell]` | Shell completions (bash, zsh, fish) |
| `fledge tui` | Interactive template browser (requires `--features tui`) |

## Remote Templates

Any GitHub repo can be a template. Use `owner/repo` syntax:

```bash
fledge init my-app --template user/my-template
fledge init my-app --template CorvidLabs/templates/python-api
fledge init my-app --template user/my-template@v1.0.0  # pin a version
fledge init my-app --template user/my-template --refresh  # force re-download
```

Register template repos so they show up in `fledge list`:

```toml
# ~/.config/fledge/config.toml
[templates]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

## Configuration

Lives at `~/.config/fledge/config.toml`:

```toml
[defaults]
author = "Your Name"
github_org = "YourOrg"
license = "MIT"

[templates]
paths = ["~/my-templates"]
repos = ["CorvidLabs/fledge-templates"]

[github]
token = "ghp_..."  # also reads FLEDGE_GITHUB_TOKEN / GITHUB_TOKEN env vars
```

If `author` isn't set, fledge pulls it from `git config user.name`.

## Creating Templates

```bash
fledge create-template my-template   # scaffold the skeleton
# edit template.toml and files
fledge init test --template ./my-template --dry-run  # test it
fledge publish ./my-template         # ship it
```

Full guide: [Template Authoring Guide](https://corvidlabs.github.io/fledge/template-authoring.html)

## License

MIT
