# fledge

Dev-lifecycle CLI — get your projects ready to fly.

A fast, opinionated CLI built in Rust. Scaffold projects, run tasks, compose workflow pipelines, manage plugins, check dependencies, review code, and ship — all from one binary.

## Why fledge?

- **Fast** — native Rust binary, no runtime dependencies
- **Smart defaults** — pulls author/org from git config, renders dates, computes name variants automatically
- **Remote templates** — use any GitHub repo as a template source with `owner/repo` syntax
- **Full lifecycle** — scaffolding, tasks, lanes, specs, CI checks, changelogs, GitHub ops, AI review
- **Composable lanes** — chain tasks into named pipelines with parallel execution
- **Plugin system** — community extensions via external executables (git-style)
- **Language-agnostic** — auto-detects Rust, Node, Go, Python, Ruby, Java and adapts defaults
- **Extensible** — create templates, plugins, and custom lane steps
- **Safe** — remote template hooks require explicit confirmation before running
- **Optional TUI** — interactive template browser with `--features tui`

## Install

```bash
# From crates.io
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
# Create a new Rust CLI project
fledge init my-tool --template rust-cli

# Browse templates interactively
fledge init my-project

# Use a remote GitHub template
fledge init my-app --template CorvidLabs/fledge-templates/react-app

# Preview what would be created
fledge init my-tool --template rust-cli --dry-run

# Run project tasks
fledge run build
fledge run test

# Compose workflow pipelines
fledge lane --init       # add default lanes
fledge lane ci           # run the CI lane
fledge lane ci --dry-run # preview execution plan

# Project health
fledge doctor            # environment diagnostics
fledge metrics           # LOC by language
fledge deps --outdated   # check for outdated deps

# Plugins
fledge plugin search deploy
fledge plugin install someone/fledge-deploy

# Check CI status
fledge checks

# Generate a changelog
fledge changelog
```

## Built-in Templates

| Template | Description |
|----------|-------------|
| `angular-app` | Angular application with mobile-first setup |
| `go-cli` | Go CLI with Cobra, Makefile, and CI |
| `monorepo` | Monorepo with workspace tooling |
| `python-cli` | Python CLI with Click, tests, and packaging |
| `rust-cli` | Rust CLI application with clap, CI, and release automation |
| `rust-lib` | Rust library crate with docs and publishing workflow |
| `swift-pkg` | Swift package with Package.swift, CI, and coding conventions |
| `ts-bun` | TypeScript project with Bun runtime |

## CLI Reference

### Scaffolding

#### `fledge init <name>`

Create a new project from a template.

```
Options:
  -t, --template      Template to use (skip interactive selection)
  -o, --output        Parent directory for the project [default: .]
      --no-git        Skip git init and initial commit
      --no-install    Skip dependency installation (post-create hooks)
      --refresh       Force re-clone of cached remote templates
      --dry-run       Show what would be created without writing anything
  -y, --yes           Skip all confirmation prompts (accept defaults)
```

#### `fledge list`

List all available templates (built-in + configured).

#### `fledge create-template <name>`

Scaffold a new fledge template with `template.toml` manifest.

```
Options:
  -o, --output        Parent directory for the template [default: .]
```

#### `fledge validate-template [path]`

Validate a template directory for correctness (manifest, Tera syntax, variable definitions, render globs).

```
Options:
      --strict          Treat warnings as errors (non-zero exit)
      --json            Output results as JSON
```

#### `fledge search [query]`

Search for templates on GitHub by keyword.

```
Options:
  -l, --limit         Maximum number of results [default: 20]
      --json          Output results as JSON
```

#### `fledge publish [path]`

Publish a template to GitHub as a repository with `fledge-template` topic.

```
Options:
      --org           Publish under a GitHub organization
      --private       Create as a private repository
      --description   Override the repository description
```

#### `fledge update`

Re-apply the source template to an existing project (update scaffolding).

```
Options:
      --dry-run       Show what would change without writing anything
      --refresh       Force re-clone of cached remote templates
  -y, --yes           Skip all confirmation prompts
```

### Project Lifecycle

#### `fledge run [task]`

Run a project task defined in `fledge.toml`. Auto-detects your project type and generates sensible defaults with `--init`.

```
Options:
      --init          Create a starter fledge.toml with language-aware defaults
  -l, --list          List available tasks
```

#### `fledge lane [name]`

Run a composable workflow pipeline. Lanes chain tasks into named pipelines with parallel execution groups.

```
Options:
  -l, --list          List available lanes
      --init          Add default lanes to fledge.toml
      --dry-run       Show execution plan without running
      --json          Output as JSON
```

#### `fledge doctor`

Diagnose project environment health (tools, config, issues).

```
Options:
      --json          Output as JSON
```

#### `fledge metrics`

Project code metrics — LOC by language, file churn, test ratio.

```
Options:
      --churn         Show most-changed files from git history
      --tests         Show test file detection and ratio
  -l, --limit         Maximum entries for churn [default: 20]
      --json          Output as JSON
```

#### `fledge deps`

Check dependency health across ecosystems.

```
Options:
      --outdated      Check for outdated dependencies
      --audit         Run security audit
      --licenses      Show dependency licenses
      --json          Output as JSON
```

#### `fledge spec <action>`

Manage spec-sync specifications (source of truth for modules).

```
Subcommands:
  check               Validate specs against source code (--strict for warnings as errors)
  init                Initialize spec-sync configuration
  new <name>          Scaffold a new spec module
```

#### `fledge work <action>`

Feature branch and PR workflow.

```
Subcommands:
  start <name>        Start a new feature branch (--base to specify base branch)
  pr                  Create a PR from current branch (--title, --body, --draft)
  status              Show current branch and PR status
```

#### `fledge changelog`

Generate a changelog from git tags and conventional commits.

```
Options:
  -l, --limit         Number of releases to show [default: 10]
  -t, --tag           Show a specific tag only
      --unreleased    Show unreleased changes since the latest tag
      --json          Output as JSON
```

### GitHub Integration

#### `fledge issues [view <number>]`

List and view GitHub issues.

```
Options:
  -s, --state         Filter by state: open, closed, all [default: open]
  -l, --limit         Maximum number of results [default: 20]
      --label         Filter by label
      --json          Output results as JSON
```

#### `fledge prs [view <number>]`

List and view GitHub pull requests.

```
Options:
  -s, --state         Filter by state: open, closed, all [default: open]
  -l, --limit         Maximum number of results [default: 20]
      --json          Output results as JSON
```

#### `fledge checks`

View CI/CD check status for a branch.

```
Options:
  -b, --branch        Branch to check [default: current branch]
      --json          Output results as JSON
```

### AI-Powered

#### `fledge review`

AI-powered code review of current changes via Claude CLI.

```
Options:
  -b, --base          Base branch to diff against [default: auto-detect]
  -f, --file          Review only a specific file
```

#### `fledge ask <question>`

Ask a question about your codebase via Claude CLI.

### Plugins

#### `fledge plugin <action>`

Manage community extensions — install, remove, list, and search.

```
Subcommands:
  install <source>     Install a plugin from GitHub (owner/repo)
  remove <name>        Remove an installed plugin
  list                 List installed plugins
  search [query]       Search for plugins on GitHub
  run <name> [args]    Run a plugin command

Options:
      --json           Output as JSON
      --force          Reinstall if already present (install only)
```

### Configuration

#### `fledge config <action>`

Manage global configuration (`~/.config/fledge/config.toml`).

```
Subcommands:
  get <key>           Get a config value
  set <key> <value>   Set a config value
  unset <key>         Remove a config value
  add <key> <value>   Add a value to a list (templates.paths, templates.repos)
  remove <key> <value> Remove a value from a list
  list                Show all config values
  path                Show config file path
  init [--preset]     Initialize config (presets: corvidlabs)
```

#### `fledge completions [shell]`

Generate or install shell completions (bash, zsh, fish, powershell).

```
Options:
      --install       Auto-install completions to the standard location
```

#### `fledge tui` *(requires `--features tui`)*

Interactive terminal UI for browsing templates and scaffolding projects.

## Remote Templates

Any GitHub repository can be a template source. Use `owner/repo` syntax:

```bash
# Use a single-template repo
fledge init my-app --template user/my-template

# Use a specific template from a collection
fledge init my-app --template CorvidLabs/templates/python-api

# Pin to a specific version/ref
fledge init my-app --template user/my-template@v1.0.0

# Force re-download of a cached template
fledge init my-app --template user/my-template --refresh
```

Remote templates are cloned and cached locally. Post-create hooks from remote templates always require confirmation unless `--yes` is passed.

### Template Repositories

Register template repos in your config so they appear in `fledge list`:

```toml
# ~/.config/fledge/config.toml
[templates]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

## Configuration

fledge reads from `~/.config/fledge/config.toml`:

```toml
[defaults]
author = "Your Name"
github_org = "YourOrg"
license = "MIT"           # default license for new projects

[templates]
paths = ["~/my-templates"]                     # additional local template directories
repos = ["CorvidLabs/fledge-templates"]         # GitHub repos to include in template list

[github]
token = "ghp_..."         # for private template repos (also reads FLEDGE_GITHUB_TOKEN / GITHUB_TOKEN env vars)
```

If `author` is not set, fledge falls back to `git config user.name`. The GitHub token is checked in order: `FLEDGE_GITHUB_TOKEN` env var -> `GITHUB_TOKEN` env var -> config file.

## Creating Templates

See the [Template Authoring Guide](https://corvidlabs.github.io/fledge/template-authoring.html) for full documentation on creating and publishing templates.

A template is a directory with a `template.toml` manifest and any number of files rendered through [Tera](https://keats.github.io/tera/) (a Jinja2-like engine).

Quick example:

```bash
# Scaffold a new template
fledge create-template my-template

# Edit template.toml and template files
# Test it
fledge init test-project --template ./my-template --dry-run

# Publish to GitHub
fledge publish ./my-template
```

## License

MIT
