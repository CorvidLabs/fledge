# CLI Reference

Every command, every flag. If it's in fledge, it's here.

## Start — Scaffold and discover

### fledge init `<name>`

Create a new project from a template.

```
fledge init <name> [OPTIONS]
```

**Arguments:**
- `<name>` — Project name

**Options:**
- `-t, --template <TEMPLATE>` — Template to use (skips interactive selection)
- `-o, --output <OUTPUT>` — Where to put it [default: `.`]
- `--no-git` — Skip git init and initial commit
- `--no-install` — Skip post-create hooks
- `--refresh` — Force re-clone of cached remote templates
- `--dry-run` — Preview without writing anything
- `-y, --yes` — Accept all defaults, skip prompts

**Examples:**

```bash
fledge init my-tool --template rust-cli
fledge init my-app --template react-app --dry-run
fledge init my-lib --template CorvidLabs/fledge-templates/rust-lib --yes
fledge init my-project --template ts-bun -o ~/projects
fledge init my-app --template CorvidLabs/templates/react-app@v2.0
```

---

### fledge list

Show all available templates — built-in, configured repos, and local paths.

```
fledge list
```

---

### fledge create-template `<name>`

Scaffold a new template directory with `template.toml` and example files.

```
fledge create-template <name> [OPTIONS]
```

**Options:**
- `-o, --output <OUTPUT>` — Parent directory [default: `.`]

---

### fledge validate-template `[path]`

Check a template for issues: manifest parsing, Tera syntax, undefined variables, glob coverage.

```
fledge validate-template [path] [OPTIONS]
```

**Arguments:**
- `[path]` — Template directory or directory of templates [default: `.`]

**Options:**
- `--strict` — Warnings become errors (non-zero exit)
- `--json` — Machine-readable output

**Examples:**

```bash
fledge validate-template ./my-template
fledge validate-template ./templates
fledge validate-template ./templates --strict   # for CI
fledge validate-template ./templates --json
```

---

### fledge search `[query]`

Find templates on GitHub (looks for the `fledge-template` topic).

```
fledge search [query] [OPTIONS]
```

**Options:**
- `-l, --limit <N>` — Max results [default: `20`]
- `--json` — JSON output

---

### fledge publish `[path]`

Push a template to GitHub as a new repo tagged with `fledge-template`.

```
fledge publish [path] [OPTIONS]
```

**Options:**
- `--org <ORG>` — Publish under an org
- `--private` — Private repo
- `--description <DESC>` — Override repo description

---

### fledge update

Re-apply the source template to an existing project. Handy when the template gets updated.

```
fledge update [OPTIONS]
```

**Options:**
- `--dry-run` — Preview changes
- `--refresh` — Force re-clone
- `-y, --yes` — Skip prompts

---

## Build — Configure and run

### fledge run `[task]`

Run tasks from `fledge.toml`. Use `--init` to auto-generate a config based on what it finds in your project.

```
fledge run [task] [OPTIONS]
```

**Options:**
- `--init` — Generate `fledge.toml` with detected defaults
- `-l, --list` — List available tasks

**Auto-detection:**

| Project | Detected by | Default tasks |
|---------|------------|---------------|
| Rust | `Cargo.toml` | build, test, clippy, fmt |
| Node.js | `package.json` | build, test, lint |
| Go | `go.mod` | build, test, vet |
| Python | `pyproject.toml` / `setup.py` | pytest, ruff, mypy |
| Ruby | `Gemfile` | rake, rspec |
| Gradle | `build.gradle` | build, test |
| Maven | `pom.xml` | compile, test |

```bash
fledge run --init
fledge run build
fledge run test
fledge run --list
```

---

### fledge lane `[name]`

Run workflow pipelines. Lanes chain tasks with parallel groups and failure control.

```
fledge lane [name] [OPTIONS]
```

**Options:**
- `-l, --list` — List lanes
- `--init` — Generate default lanes
- `--dry-run` — Preview the plan
- `--json` — JSON output

**Lane config in fledge.toml:**

```toml
[lanes.ci]
description = "Full CI pipeline"
steps = ["lint", "test", "build"]

[lanes.check]
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]

[lanes.release]
fail_fast = false
steps = [
  "test",
  { run = "cargo build --release" },
  "publish"
]
```

**Step types:**

| Type | Syntax | |
|------|--------|-|
| Task reference | `"task_name"` | Runs a task from `[tasks]` |
| Inline command | `{ run = "command" }` | Shell command |
| Parallel group | `{ parallel = ["a", "b"] }` | Concurrent execution |

```bash
fledge lane
fledge lane ci
fledge lane ci --dry-run
fledge lane --init
```

---

### fledge config `<action>`

Manage `~/.config/fledge/config.toml`.

```
fledge config <get|set|unset|add|remove|list|path|init>
```

| Subcommand | What it does |
|------------|-------------|
| `get <key>` | Read a value |
| `set <key> <value>` | Write a value |
| `unset <key>` | Delete a value |
| `add <key> <value>` | Append to a list (`templates.paths`, `templates.repos`) |
| `remove <key> <value>` | Remove from a list |
| `list` | Show everything |
| `path` | Print config file path |
| `init [--preset <name>]` | Initialize config (presets: `corvidlabs`) |

**Valid keys:**
- `defaults.author`, `defaults.github_org`, `defaults.license`
- `github.token`
- `templates.paths`, `templates.repos`

```bash
fledge config set defaults.author "Leif"
fledge config add templates.repos "CorvidLabs/fledge-templates"
fledge config list
```

---

### fledge doctor

Check your environment for issues (missing tools, bad config, etc). Run this before `fledge run` if something seems off.

```
fledge doctor [OPTIONS]
```

**Options:**
- `--json` — JSON output

---

## Develop — Branch and spec

### fledge work `<action>`

Feature branch and PR workflow.

```
fledge work <start|pr|status> [OPTIONS]
```

**Subcommands:**

- `start <name>` — Create `feat/<name>` branch (`--base` to pick the base)
- `pr` — Open a PR (`--title`, `--body`, `--draft`, `--base`)
- `status` — Current branch + PR status

---

### fledge spec `<action>`

Spec-sync management. Specs are the source of truth for module design.

```
fledge spec <check|init|new> [OPTIONS]
```

**Subcommands:**

- `check` — Validate specs against code (`--strict` for warnings as errors)
- `init` — Set up spec-sync for the project
- `new <name>` — Scaffold a new spec

---

## Review — Quality and insight

### fledge review

AI code review via Claude. Diffs your branch against the base and gives feedback.

```
fledge review [OPTIONS]
```

**Options:**
- `-b, --base <BRANCH>` — Base branch [default: auto-detect]
- `-f, --file <FILE>` — Review a single file

---

### fledge ask `<question>`

Ask about your codebase. Claude reads your code and answers.

```
fledge ask <question>
```

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
```

---

### fledge metrics

Code stats — LOC by language, file churn, test ratio.

```
fledge metrics [OPTIONS]
```

**Options:**
- `--churn` — Most-changed files from git history
- `--tests` — Test file detection and ratio
- `-l, --limit <N>` — Max churn entries [default: `20`]
- `--json` — JSON output

```bash
fledge metrics
fledge metrics --churn
fledge metrics --tests
fledge metrics --churn --tests --json
```

---

### fledge deps

Dependency health checks.

```
fledge deps [OPTIONS]
```

**Options:**
- `--outdated` — Find stale dependencies
- `--audit` — Security audit
- `--licenses` — License scan
- `--json` — JSON output

**Works with:**

| Ecosystem | Detected by | Outdated | Audit | Licenses |
|-----------|------------|----------|-------|----------|
| Rust | `Cargo.lock` | `cargo outdated` | `cargo audit` | `cargo license` |
| Node.js | `package-lock.json` / `yarn.lock` | npm/yarn outdated | npm/yarn audit | `license-checker` |
| Go | `go.sum` | `go list` | `govulncheck` | — |
| Python | `requirements.txt` / `Pipfile.lock` / `poetry.lock` | pip outdated | `pip-audit` | — |
| Ruby | `Gemfile.lock` | `bundle outdated` | `bundle audit` | — |

```bash
fledge deps
fledge deps --outdated
fledge deps --audit
fledge deps --outdated --audit --licenses --json
```

---

## Ship — Track and release

### fledge issues `[view <number>]`

List and view GitHub issues.

```
fledge issues [OPTIONS]
fledge issues view <number> [OPTIONS]
```

**Options:**
- `-s, --state <STATE>` — `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>` — Max results [default: `20`]
- `--label <LABEL>` — Filter by label
- `--json`

---

### fledge prs `[view <number>]`

List and view pull requests.

```
fledge prs [OPTIONS]
fledge prs view <number> [OPTIONS]
```

**Options:**
- `-s, --state <STATE>` — `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>` — Max results [default: `20`]
- `--json`

---

### fledge checks

CI/CD status for a branch.

```
fledge checks [OPTIONS]
```

**Options:**
- `-b, --branch <BRANCH>` — Branch to check [default: current]
- `--json`

---

### fledge changelog

Generate a changelog from git tags and conventional commits.

```
fledge changelog [OPTIONS]
```

**Options:**
- `-l, --limit <N>` — Releases to show [default: `10`]
- `-t, --tag <TAG>` — Specific tag
- `--unreleased` — Changes since last tag
- `--json` — JSON output

```bash
fledge changelog
fledge changelog --unreleased
fledge changelog --json
fledge changelog --tag v0.7.0
```

---

## Extend — Grow the tool

### fledge plugin `<action>`

Install, manage, and run community plugins.

```
fledge plugin <install|remove|list|search|run> [OPTIONS]
```

**Subcommands:**

- `install <source>` — Install from GitHub (`owner/repo` or URL). `--force` to reinstall.
- `remove <name>` — Uninstall a plugin
- `list` — Show installed plugins
- `search [query]` — Find plugins on GitHub (`--limit`)
- `run <name> [args...]` — Run a plugin command

`--json` works with `list` and `search`.

**Plugin format** (`plugin.toml`):

```toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
description = "Deploy to cloud providers"
author = "someone"

[[commands]]
name = "deploy"
description = "Deploy the project"
binary = "fledge-deploy"

[[hooks]]
event = "lane:post"
binary = "fledge-deploy-notify"
```

```bash
fledge plugin install someone/fledge-deploy
fledge plugin list
fledge plugin search deploy
fledge plugin remove fledge-deploy
```

---

### fledge completions `[shell]`

Shell completions for bash, zsh, fish, powershell.

```
fledge completions [shell] [OPTIONS]
```

**Options:**
- `--install` — Auto-install for your current shell

```bash
fledge completions --install
fledge completions bash >> ~/.bashrc
fledge completions zsh > ~/.zfunc/_fledge
fledge completions fish > ~/.config/fish/completions/fledge.fish
```

---

### fledge tui *(requires `--features tui`)*

Interactive template browser. Browse, preview, and scaffold templates without memorizing command flags.

```
fledge tui [OPTIONS]
```

**Options:**
- `-o, --output <OUTPUT>` — Where to put the project [default: `.`]
- `--no-git` — Skip git init

**Navigation:** Arrow keys to browse, Tab to fill in variables, Enter to create.
