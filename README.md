# fledge

Get your projects ready to fly.

A fast, opinionated dev-lifecycle CLI built in Rust. Scaffold projects from templates, manage specs, run tasks, check CI, review code, and ship — all from one binary.

## Why fledge?

- **Fast** — native Rust binary, no runtime dependencies
- **Smart defaults** — pulls author/org from git config, renders dates, computes name variants automatically
- **Remote templates** — use any GitHub repo as a template source with `owner/repo` syntax
- **Full lifecycle** — scaffolding, specs, tasks, CI checks, changelogs, GitHub ops, AI review
- **Language-agnostic** — auto-detects Rust, Node, Go, Python, Ruby, Java and adapts defaults
- **Extensible** — create your own templates with a simple `template.toml` manifest
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

# Check CI status
fledge checks

# Generate a changelog
fledge changelog
```

## Built-in Templates

| Template | Description |
|----------|-------------|
| `rust-cli` | Rust CLI application with clap, CI, and release automation |
| `rust-lib` | Rust library crate with docs and publishing workflow |
| `swift-pkg` | Swift package with Package.swift, CI, and coding conventions |
| `ts-bun` | TypeScript project with Bun runtime |
| `angular-app` | Angular application with mobile-first setup |
| `python-cli` | Python CLI with Click, tests, and packaging |
| `go-cli` | Go CLI with Cobra, Makefile, and CI |
| `node-cli` | Node.js CLI with TypeScript |
| `node-lib` | Node.js library with TypeScript and npm publishing |
| `monorepo` | Monorepo with workspace tooling |
| `static-site` | Static site with build pipeline |

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
