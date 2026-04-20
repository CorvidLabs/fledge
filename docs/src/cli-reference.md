# CLI Reference

Complete reference for all fledge commands and options.

## Scaffolding Commands

### fledge init `<name>`

Create a new project from a template.

#### Usage

```
fledge init <name> [OPTIONS]
```

#### Arguments

- `<name>` — Project name

#### Options

- `-t, --template <TEMPLATE>` — Template to use (skip interactive selection)
- `-o, --output <OUTPUT>` — Parent directory for the project [default: `.`]
- `--no-git` — Skip git init and initial commit
- `--no-install` — Skip dependency installation (post-create hooks)
- `--refresh` — Force re-clone of cached remote templates
- `--dry-run` — Show what would be created without writing anything
- `-y, --yes` — Skip all confirmation prompts (accept defaults)

#### Examples

```bash
# Create with defaults
fledge init my-tool --template rust-cli

# Preview before creating
fledge init my-app --template react-app --dry-run

# Skip all prompts
fledge init my-lib --template rust-lib --yes

# Specify output directory
fledge init my-project --template ts-bun -o ~/projects

# Use a remote template pinned to a version
fledge init my-app --template CorvidLabs/templates/react-app@v2.0
```

---

### fledge list

List all available templates (built-in + configured).

#### Usage

```
fledge list
```

Shows template name, description, and source (built-in or configured repo).

---

### fledge create-template `<name>`

Scaffold a new fledge template with a `template.toml` manifest and example files.

#### Usage

```
fledge create-template <name> [OPTIONS]
```

#### Arguments

- `<name>` — Template name

#### Options

- `-o, --output <OUTPUT>` — Parent directory for the template [default: `.`]

---

### fledge search `[query]`

Search for templates on GitHub using the `fledge-template` topic.

#### Usage

```
fledge search [query] [OPTIONS]
```

#### Arguments

- `[query]` — Keyword to filter results (optional)

#### Options

- `-l, --limit <LIMIT>` — Maximum number of results [default: `20`]
- `--json` — Output results as JSON

---

### fledge publish `[path]`

Publish a template directory to GitHub as a new repository tagged with `fledge-template`.

#### Usage

```
fledge publish [path] [OPTIONS]
```

#### Arguments

- `[path]` — Path to the template directory [default: `.`]

#### Options

- `--org <ORG>` — Publish under a GitHub organization
- `--private` — Create as a private repository
- `--description <DESC>` — Override the repository description

---

### fledge update

Re-apply the source template to an existing project. Useful when the template has been updated and you want to pull in changes.

#### Usage

```
fledge update [OPTIONS]
```

#### Options

- `--dry-run` — Show what would change without writing anything
- `--refresh` — Force re-clone of cached remote templates
- `-y, --yes` — Skip all confirmation prompts

---

## Project Lifecycle Commands

### fledge run `[task]`

Run a project task defined in `fledge.toml`. If no task is specified, lists available tasks. Use `--init` to generate a starter `fledge.toml` with language-aware defaults for your project type.

#### Usage

```
fledge run [task] [OPTIONS]
```

#### Arguments

- `[task]` — Task name to run (lists tasks if omitted)

#### Options

- `--init` — Create a starter `fledge.toml` with detected project defaults
- `-l, --list` — List available tasks

#### Supported Project Types

`fledge run --init` auto-detects your project and generates appropriate task definitions:

| Project Type | Detection | Default Tasks |
|--------------|-----------|---------------|
| Rust | `Cargo.toml` | `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` |
| Node.js | `package.json` | `npm run build`, `npm test`, `npm run lint` |
| Go | `go.mod` | `go build`, `go test`, `go vet` |
| Python | `pyproject.toml` / `setup.py` | `pytest`, `ruff check`, `mypy` |
| Ruby | `Gemfile` | `bundle exec rake`, `bundle exec rspec` |
| Gradle | `build.gradle` | `./gradlew build`, `./gradlew test` |
| Maven | `pom.xml` | `mvn compile`, `mvn test` |

#### Examples

```bash
# Initialize task config
fledge run --init

# Run a task
fledge run build
fledge run test

# List available tasks
fledge run --list
```

---

### fledge spec `<action>`

Manage spec-sync specifications. Specs are the source of truth for module design and implementation.

#### Usage

```
fledge spec <check|init|new> [OPTIONS]
```

#### Subcommands

##### `fledge spec check`

Validate all specs against the source code.

- `--strict` — Treat warnings as errors

##### `fledge spec init`

Initialize spec-sync configuration for the project.

##### `fledge spec new <name>`

Scaffold a new spec module with all required sections.

---

### fledge work `<action>`

Feature branch and PR workflow — start branches, create PRs, and check status.

#### Usage

```
fledge work <start|pr|status> [OPTIONS]
```

#### Subcommands

##### `fledge work start <name>`

Start a new feature branch. The name is sanitized and prefixed with `feat/`.

- `--base <BRANCH>` — Base branch to branch from [default: `main`]

##### `fledge work pr`

Create a pull request from the current branch.

- `-t, --title <TITLE>` — PR title (auto-generated from branch name if omitted)
- `-b, --body <BODY>` — PR body/description
- `--draft` — Create as a draft PR
- `--base <BRANCH>` — Target base branch for the PR

##### `fledge work status`

Show the current branch name and associated PR status.

---

### fledge changelog

Generate a changelog from git tags and conventional commits. Groups commits by type (features, fixes, etc.).

#### Usage

```
fledge changelog [OPTIONS]
```

#### Options

- `-l, --limit <N>` — Number of releases to show [default: `10`]
- `-t, --tag <TAG>` — Show a specific tag only
- `--unreleased` — Show unreleased changes since the latest tag
- `--json` — Output as JSON

#### Examples

```bash
# Show recent releases
fledge changelog

# Show only unreleased changes
fledge changelog --unreleased

# Export as JSON for automation
fledge changelog --json

# Show a specific release
fledge changelog --tag v0.7.0
```

---

## GitHub Integration Commands

### fledge issues `[view <number>]`

List and view GitHub issues for the current repository.

#### Usage

```
fledge issues [OPTIONS]
fledge issues view <number> [OPTIONS]
```

#### Options

- `-s, --state <STATE>` — Filter by state: `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>` — Maximum number of results [default: `20`]
- `--label <LABEL>` — Filter by label
- `--json` — Output results as JSON

---

### fledge prs `[view <number>]`

List and view GitHub pull requests for the current repository.

#### Usage

```
fledge prs [OPTIONS]
fledge prs view <number> [OPTIONS]
```

#### Options

- `-s, --state <STATE>` — Filter by state: `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>` — Maximum number of results [default: `20`]
- `--json` — Output results as JSON

---

### fledge checks

View CI/CD check status for a branch.

#### Usage

```
fledge checks [OPTIONS]
```

#### Options

- `-b, --branch <BRANCH>` — Branch to check [default: current branch]
- `--json` — Output results as JSON

---

## AI-Powered Commands

### fledge review

AI-powered code review of current changes using Claude CLI. Diffs the current branch against the base branch and provides review feedback.

#### Usage

```
fledge review [OPTIONS]
```

#### Options

- `-b, --base <BRANCH>` — Base branch to diff against [default: auto-detect]
- `-f, --file <FILE>` — Review only a specific file

---

### fledge ask `<question>`

Ask a question about your codebase using Claude CLI. Provides context-aware answers based on your project's source code.

#### Usage

```
fledge ask <question>
```

#### Examples

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
```

---

## Configuration Commands

### fledge config `<action>`

Manage global configuration stored in `~/.config/fledge/config.toml`.

#### Usage

```
fledge config <get|set|unset|add|remove|list|path>
```

#### Subcommands

| Subcommand | Description |
|------------|-------------|
| `get <key>` | Get a config value |
| `set <key> <value>` | Set a config value |
| `unset <key>` | Remove a config value |
| `add <key> <value>` | Add a value to a list key (`templates.paths`, `templates.repos`) |
| `remove <key> <value>` | Remove a value from a list key |
| `list` | Show all config values |
| `path` | Show config file path |

#### Valid Keys

- `defaults.author` — Default author name
- `defaults.github_org` — Default GitHub organization
- `defaults.license` — Default license
- `github.token` — GitHub personal access token
- `templates.paths` — Local template directories (list)
- `templates.repos` — GitHub template repositories (list)

#### Examples

```bash
fledge config set defaults.author "Leif"
fledge config add templates.repos "CorvidLabs/fledge-templates"
fledge config list
```

---

### fledge completions `[shell]`

Generate or install shell completions.

#### Usage

```
fledge completions [shell] [OPTIONS]
```

#### Arguments

- `[shell]` — Shell to generate completions for: `bash`, `zsh`, `fish`, `powershell` (auto-detects with `--install`)

#### Options

- `--install` — Install completions to the standard location for your shell

#### Examples

```bash
# Auto-install for your current shell
fledge completions --install

# Manual generation
fledge completions bash >> ~/.bashrc
fledge completions zsh > ~/.zfunc/_fledge
fledge completions fish > ~/.config/fish/completions/fledge.fish
```

---

### fledge tui *(requires `--features tui`)*

Interactive terminal UI for browsing templates and scaffolding projects.

#### Usage

```
fledge tui [OPTIONS]
```

#### Options

- `-o, --output <OUTPUT>` — Parent directory for the project [default: `.`]
- `--no-git` — Skip git init and initial commit

#### Navigation

- **Arrow keys** — Browse templates
- **Tab** — Fill in project variables
- **Enter** — Confirm and create
