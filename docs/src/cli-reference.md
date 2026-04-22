# CLI Reference

Every command, every flag. If it's in fledge, it's here.

**Jump to:**
[Start](#start-scaffold-and-discover) |
[Build](#build-configure-and-run) |
[Develop](#develop-branch-and-spec) |
[Review](#review-quality-and-insight) |
[Ship](#ship-track-and-release) |
[Extend](#extend-grow-the-tool)

## Start: Scaffold and discover

All template commands live under `fledge templates` (alias: `fledge template`).

### fledge templates init `<name>`

Create a new project from a template.

```
fledge templates init <name> [OPTIONS]
```

**Arguments:**
- `<name>` - Project name

**Options:**
- `-t, --template <TEMPLATE>` - Template to use (skips interactive selection)
- `-o, --output <OUTPUT>` - Where to put it [default: `.`]
- `--author <AUTHOR>` - Author name (bypasses prompt; overrides config)
- `--org <ORG>` - GitHub organization (bypasses prompt; overrides config)
- `--no-git` - Skip git init and initial commit
- `--no-install` - Skip post-create hooks
- `--refresh` - Force re-clone of cached remote templates
- `--dry-run` - Preview without writing anything
- `-y, --yes` - Accept all defaults, skip prompts

**Behavior:**

- If the template specifies `min_fledge_version`, fledge checks compatibility before proceeding.
- If the template specifies `requires` (tool dependencies), fledge checks they're on PATH and warns about missing ones.
- If the template doesn't include a `fledge.toml`, one is auto-generated from detected project type defaults.
- If no git user is configured, defaults to `user.name=fledge` and `user.email=fledge@localhost`.
- Plugin `pre_init` lifecycle hooks run before template rendering (if any plugins define them).

**Examples:**

```bash
fledge templates init my-tool --template rust-cli
fledge templates init my-app --template ts-bun --dry-run
fledge templates init my-lib --template go-cli --yes
fledge templates init my-project --template python-cli -o ~/projects
fledge templates init my-app --template CorvidLabs/fledge-templates/deno-cli@v2.0
fledge templates init my-tool --template rust-cli --author "Leif" --org CorvidLabs --yes
```

---

### fledge templates list

Show all available templates (built-in, configured repos, and local paths).

```
fledge templates list
```

---

### fledge templates create `<name>`

Scaffold a new template directory with `template.toml` and example files.

```
fledge templates create <name> [OPTIONS]
```

**Options:**
- `-o, --output <OUTPUT>` - Parent directory [default: `.`]
- `-d, --description <DESC>` - Template description (bypasses prompt)
- `--render-patterns <PATTERNS>` - Comma-separated file patterns to render through Tera (bypasses prompt)
- `--hooks [BOOL]` - Include post-create hooks scaffold (bypasses prompt; accepts optional `true`/`false`, defaults to `true`)
- `--prompts [BOOL]` - Include custom prompts scaffold (bypasses prompt; accepts optional `true`/`false`, defaults to `true`)
- `-y, --yes` - Skip all interactive prompts (accept defaults)

**Examples:**

```bash
fledge templates create my-template
fledge templates create my-template -d "FastAPI starter" --render-patterns "**/*.py,**/*.toml" --hooks --yes
```

---

### fledge templates validate `[path]`

Check a template for issues: manifest parsing, Tera syntax, undefined variables, glob coverage. GitHub Actions `${{ }}` syntax is automatically filtered out so it isn't flagged as Tera.

```
fledge templates validate [path] [OPTIONS]
```

**Arguments:**
- `[path]` - Template directory or directory of templates [default: `.`]

**Options:**
- `--strict` - Warnings become errors (non-zero exit)
- `--json` - Machine-readable output

**Examples:**

```bash
fledge templates validate ./my-template
fledge templates validate ./templates
fledge templates validate ./templates --strict   # for CI
fledge templates validate ./templates --json
```

---

### fledge templates search `[query]`

Find templates on GitHub (looks for the `fledge-template` topic).

```
fledge templates search [query] [OPTIONS]
```

**Options:**
- `-a, --author <OWNER>` - Filter by author/owner
- `-l, --limit <N>` - Max results [default: `20`, max: `100`]
- `--json` - JSON output

**Output format:** Results show repo name, star count (abbreviated with "k" suffix for 1000+), and description (truncated to 60 characters). Topics are shown below each result.

---

### fledge templates publish `[path]`

Push a template to GitHub as a new repo tagged with `fledge-template`.

```
fledge templates publish [path] [OPTIONS]
```

**Options:**
- `--org <ORG>` - Publish under an org
- `--private` - Private repo
- `--description <DESC>` - Override repo description (falls back to description in `template.toml`)

**Behavior:**

- If the repo already exists on GitHub, prompts for confirmation before updating.
- Auto-initializes git (`.git`) if the template directory doesn't have one.
- Cleans up embedded tokens from the remote URL after push.

---

### fledge templates update

Re-apply the source template to an existing project. Handy when the template gets updated.

```
fledge templates update [OPTIONS]
```

**Options:**
- `--dry-run` - Preview changes
- `--refresh` - Force re-clone

---

## Build: Configure and run

### fledge run `[task]`

Run tasks. Works with zero config (auto-detects your project type) or from `fledge.toml` when you want full control.

```
fledge run [task] [OPTIONS]
```

**Options:**
- `--init` - Generate `fledge.toml` with detected defaults
- `-l, --list` - List available tasks
- `--lang <LANG>` - Override detected project language (swift, python, rust, node, go)

**Zero-config mode** (no `fledge.toml`): Fledge detects your project type from marker files and provides default tasks automatically. For Node.js projects, it also detects your package manager (npm, bun, yarn, pnpm) from lockfiles.

**Config mode** (`fledge.toml` exists): The config file takes full precedence. No mixing with auto-detection.

**Auto-detection:**

| Project | Detected by | Default tasks |
|---------|------------|---------------|
| Rust | `Cargo.toml` | build, test, clippy, fmt |
| Node.js | `package.json` | build, test, lint, dev (if scripts exist) |
| Go | `go.mod` | build, test, vet |
| Python | `pyproject.toml` / `setup.py` | pytest, ruff, mypy |
| Ruby | `Gemfile` | rake, rspec |
| Swift | `Package.swift` | build, test |
| Gradle | `build.gradle` | build, test |
| Maven | `pom.xml` | compile, test |

```bash
fledge run test          # works immediately in any detected project
fledge run --list        # see what's available
fledge run --init        # generate fledge.toml to customize
fledge run build         # run a specific task
fledge run test --lang swift  # override detected language
```

---

### fledge lanes

Manage and run composable workflow pipelines. Alias: `fledge lane`.

```
fledge lanes <run|list|init|search|import|publish|create|validate>
```

**Subcommands:**

- `run <name>` - Run a lane by name (`--dry-run` to preview)
- `list` - List available lanes (`--json` for JSON output)
- `init` - Add default lanes to `fledge.toml`
- `search [query]` - Search GitHub for community lanes (`--author`, `--json`)
- `import <source>` - Import lanes from a GitHub repo (owner/repo or owner/repo@ref)
- `publish [path]` - Publish lanes to GitHub (`--org`, `--private`, `--description`)
- `create <name>` - Scaffold a new lane repo (`--output`, `--description`, `--yes`)
- `validate [path]` - Validate lane definitions in fledge.toml (`--strict`, `--json`)

**Shortcut:** `fledge lane ci` is equivalent to `fledge lanes run ci`.

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
fledge lane ci                # run a lane (shortcut)
fledge lanes run ci           # same thing, explicit
fledge lanes run ci --dry-run
fledge lanes list
fledge lanes list --json
fledge lanes init
fledge lanes search
fledge lanes search rust
fledge lanes import CorvidLabs/fledge-lanes
fledge lanes publish --org MyOrg
fledge lanes create my-lanes
fledge lanes create my-lanes --yes --description "My CI lanes"
fledge lanes validate
fledge lanes validate ./my-lanes --strict
fledge lanes validate --json
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
- `--json` - JSON output

---

## Develop: Branch and spec

### fledge work `<action>`

Work branch and PR workflow. Supports any branch type, not just features.

```
fledge work <start|pr|status> [OPTIONS]
```

**Subcommands:**

- `start <name>` - Create a work branch
- `pr` - Open a PR (`-t, --title`, `-b, --body`, `--draft`, `--base`)
- `status` - Current branch + PR status

**Options for `work start`:**

- `-t, --branch-type <TYPE>` - Branch type: `feat`, `fix`, `chore`, `docs`, `hotfix`, `refactor` [default: `feat`]
- `-i, --issue <NUMBER>` - Link to GitHub issue (prefixes branch name with issue number)
- `--prefix <PREFIX>` - Override branch prefix entirely (e.g. `user/leif`)
- `--base <BRANCH>` - Base branch [default: `main`]

The branch format is configurable via `[work]` in `fledge.toml`:

```toml
[work]
default_type = "feat"
branch_format = "{author}/{type}/{name}"
```

**Examples:**

```bash
fledge work start add-auth                    # leif/feat/add-auth (default: {author}/{type}/{name})
fledge work start login-crash --branch-type fix      # leif/fix/login-crash
fledge work start bump-deps --branch-type chore      # leif/chore/bump-deps
fledge work start login-crash --issue 42      # leif/feat/42-login-crash
fledge work start my-feature --prefix user/leif  # user/leif/my-feature
```

---

### fledge spec `<action>`

Spec-sync management. Specs are the source of truth for module design.

```
fledge spec <check|init|new> [OPTIONS]
```

**Subcommands:**

- `check` - Validate specs against code (`--strict` for warnings as errors)
- `init` - Set up spec-sync for the project
- `new <name>` - Scaffold a new spec

---

## Review: Quality and insight

### fledge review

AI code review via Claude. Diffs your branch against the base and gives feedback.

```
fledge review [OPTIONS]
```

**Options:**
- `-b, --base <BRANCH>` - Base branch [default: auto-detect]
- `-f, --file <FILE>` - Review a single file
- `--json` - JSON output

**Default branch detection:** When `--base` is not specified, fledge tries `git symbolic-ref refs/remotes/origin/HEAD`, then checks for `main` and `master` branches. Falls back to `main` if none exist.

---

### fledge ask `<question>`

Ask about your codebase. Claude reads your code and answers.

```
fledge ask <question> [OPTIONS]
```

**Options:**
- `--json` - JSON output

```bash
fledge ask "how does the template rendering work?"
fledge ask "what tests cover the config module?"
```

---

### fledge metrics

Code stats: LOC by language, file churn, test ratio.

```
fledge metrics [OPTIONS]
```

**Options:**
- `--churn` - Most-changed files from git history
- `--tests` - Test file detection and ratio
- `-l, --limit <N>` - Max churn entries [default: `20`]
- `--json` - JSON output

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
- `--outdated` - Find stale dependencies
- `--audit` - Security audit
- `--licenses` - License scan
- `--json` - JSON output

**Works with:**

| Ecosystem | Detected by | Outdated | Audit | Licenses |
|-----------|------------|----------|-------|----------|
| Rust | `Cargo.lock` | `cargo outdated` | `cargo audit` | `cargo license` |
| Node.js | `package-lock.json` / `yarn.lock` | npm/yarn outdated | npm/yarn audit | `license-checker` |
| Go | `go.sum` | `go list` | `govulncheck` | N/A |
| Python | `requirements.txt` / `Pipfile.lock` / `poetry.lock` | pip outdated | `pip-audit` | N/A |
| Ruby | `Gemfile.lock` | `bundle outdated` | `bundle audit` | N/A |

```bash
fledge deps
fledge deps --outdated
fledge deps --audit
fledge deps --outdated --audit --licenses --json
```

---

## Ship: Track and release

### fledge issues `[view <number>]`

List and view GitHub issues.

```
fledge issues [OPTIONS]
fledge issues view <number> [OPTIONS]
```

**Options:**
- `-s, --state <STATE>` - `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>` - Max results [default: `20`]
- `--label <LABEL>` - Filter by label
- `--json`

---

### fledge prs `[view <number>]`

List and view pull requests.

```
fledge prs [OPTIONS]
fledge prs view <number> [OPTIONS]
```

**Options:**
- `-s, --state <STATE>` - `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>` - Max results [default: `20`]
- `--json`

---

### fledge checks

CI/CD status for a branch.

```
fledge checks [OPTIONS]
```

**Options:**
- `-b, --branch <BRANCH>` - Branch to check [default: current]
- `--json`

---

### fledge changelog

Generate a changelog from git tags and conventional commits.

```
fledge changelog [OPTIONS]
```

**Options:**
- `-l, --limit <N>` - Releases to show [default: `10`]
- `-t, --tag <TAG>` - Specific tag
- `--unreleased` - Changes since last tag
- `--json` - JSON output

```bash
fledge changelog
fledge changelog --unreleased
fledge changelog --json
fledge changelog --tag v0.7.0
```

---

### fledge release `<bump>`

Cut a release — bump version, generate changelog, create a git tag, and optionally push.

```
fledge release <bump> [OPTIONS]
```

**Arguments:**
- `<bump>` - Version bump: `major`, `minor`, `patch`, or an explicit version (e.g. `1.0.0`)

**Options:**
- `--dry-run` - Show what would happen without making changes
- `--no-tag` - Skip creating a git tag
- `--no-changelog` - Skip changelog generation
- `--push` - Push commit and tag to remote after release
- `--pre-lane <NAME>` - Run a lane before releasing (e.g. `ci`)
- `--allow-dirty` - Allow releasing with uncommitted changes

**Examples:**

```bash
fledge release patch                          # bump patch version
fledge release minor --push                   # bump minor + push
fledge release major --pre-lane ci            # run CI lane first, then bump major
fledge release 2.0.0 --dry-run               # preview a specific version bump
fledge release patch --no-tag --no-changelog  # just bump version
```

---

## Extend: Grow the tool

### fledge plugins `<action>`

Install, manage, and run community plugins. Alias: `fledge plugin`.

```
fledge plugins <install|remove|update|list|search|run|publish|create|validate> [OPTIONS]
```

**Subcommands:**

- `install <source[@ref]>` - Install from GitHub (`owner/repo[@tag]` or URL). `--force` to reinstall. Use `@ref` to pin to a tag, branch, or commit.
- `remove <name>` - Uninstall a plugin
- `update [name]` - Update plugins. Unpinned plugins get `git pull`; pinned plugins check for newer tags.
- `list` - Show installed plugins (includes pinned version info)
- `search [query]` - Find plugins on GitHub (`--author`, `--limit`)
- `run <name> [args...]` - Run a plugin command
- `publish [path]` - Publish a plugin to GitHub (`--org`, `--private`, `--description`)
- `create <name>` - Scaffold a new plugin (`--output`, `--description`, `--yes`)
- `validate [path]` - Validate a plugin manifest (`--strict`, `--json`)

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

[hooks]
build = "cargo build --release"
post_install = "echo 'Deploy plugin ready'"
```

```bash
fledge plugins install someone/fledge-deploy
fledge plugins install someone/fledge-deploy@v1.2.0   # pin to version
fledge plugins update                                   # update all
fledge plugins list
fledge plugins search deploy
fledge plugins remove fledge-deploy
fledge plugins publish --org MyOrg
fledge plugins create my-tool
fledge plugins create my-tool --yes --description "My deploy tool"
fledge plugins validate
fledge plugins validate ./my-tool --strict
fledge plugins validate --json
```

---

### fledge completions `[shell]`

Shell completions for bash, zsh, fish, powershell.

```
fledge completions [shell] [OPTIONS]
```

**Options:**
- `--install` - Auto-install for your current shell

```bash
fledge completions --install
fledge completions bash >> ~/.bashrc
fledge completions zsh > ~/.zfunc/_fledge
fledge completions fish > ~/.config/fish/completions/fledge.fish
```

---
