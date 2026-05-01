# CLI Reference

Every command, every flag. If it's in fledge core, it's here. Plugin commands (`github checks`, `github issues`, `github prs`, `deps`, `metrics`) ship as separate repos, install them with `fledge plugins install --defaults`.

**Jump to:**
[Scaffold](#scaffold-templates) |
[Run](#run-tasks-and-lanes) |
[Spec](#spec-spec-sync) |
[AI](#ai-ask-and-review) |
[Ship](#ship-branch-pr-release) |
[Extend](#extend-plugins-config-tools)

## Scaffold: Templates

All template commands live under `fledge templates` (alias: `fledge template`). Six subcommands: `init`, `create`, `validate`, `list`, `search`, `publish`.

### fledge templates init `<name>`

Create a new project from a template.

```text
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
- Plugin `pre_init` lifecycle hooks run before config loading (if any plugins define them).

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

```text
fledge templates list [OPTIONS]
```

**Options:**
- `--json` - JSON output

---

### fledge templates create `<name>`

Scaffold a new template directory with `template.toml` and example files.

```text
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

```text
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

Find templates on GitHub by searching for the `fledge-template` topic.

```text
fledge templates search [query] [OPTIONS]
```

**Options:**
- `-a, --author <OWNER>` - Filter by author/owner
- `-l, --limit <N>` - Max results [default: `20`, max: `100`]
- `--json` - JSON output

**Output (`--json`):** array of `{owner, name, description, stars, url, topics, trust_tier}`.

---

### fledge templates publish `[path]`

Push a template directory to GitHub as a new repo tagged with the `fledge-template` topic. Validates the template via the same gate `fledge templates validate` uses, then creates the repo (or updates an existing one), sets the topic, and force-pushes the directory contents.

```text
fledge templates publish [path] [OPTIONS]
```

**Options:**
- `--org <ORG>` - Publish under an org
- `--private` - Private repo
- `--description <DESC>` - Override repo description
- `-y, --yes` - Skip confirmation prompt (also auto-promoted by `FLEDGE_NON_INTERACTIVE=1`)

---

## Run: Tasks and Lanes

### fledge run `[task]`

Run tasks. Works with zero config (auto-detects your project type) or from `fledge.toml` when you want full control.

```text
fledge run [task] [OPTIONS]
```

**Options:**
- `--init` - Generate `fledge.toml` with detected defaults
- `-l, --list` - List available tasks
- `--lang <LANG>` - Override detected project language (swift, python, rust, node, go, ruby, java-gradle, java-maven)
- `--json` - JSON output (works with `--list` and when running a task)

**Zero-config mode** (no `fledge.toml`): Fledge detects your project type from marker files and provides default tasks automatically. For Node.js projects, it also detects your package manager (npm, bun, yarn, pnpm) from lockfiles.

**Config mode** (`fledge.toml` exists): The config file takes full precedence. No mixing with auto-detection.

**Auto-detection:**

| Project | Detected by | Default tasks |
|---------|------------|---------------|
| Rust | `Cargo.toml` | build, test, lint, fmt |
| Node.js | `package.json` | test, build, lint, dev (if scripts exist) |
| Go | `go.mod` | build, test, lint |
| Python | `pyproject.toml` / `setup.py` | test, lint, fmt |
| Ruby | `Gemfile` | test, lint |
| Swift | `Package.swift` | build, test |
| Gradle | `build.gradle` | build, test |
| Maven | `pom.xml` | build, test |

```bash
fledge run test          # works immediately in any detected project
fledge run --list        # see what's available
fledge run --init        # generate fledge.toml to customize
fledge run build         # run a specific task
fledge run test --lang swift  # override detected language
```

---

### fledge lanes

Manage and run composable workflow pipelines.

```text
fledge lanes <run|list|init|search|import|publish|create|validate>
```

**Subcommands:**

- `run <name>` - Run a lane by name (`--dry-run` to preview, `--json` for JSON output)
- `list` - List available lanes (`--json` for JSON output)
- `init` - Add default lanes to `fledge.toml`
- `search [query]` - Search GitHub for community lanes (`--author`, `--json`)
- `import <source>` - Import lanes from a GitHub repo (owner/repo or owner/repo@ref)
- `publish [path]` - Publish lanes to GitHub (`--org`, `--private`, `--description`)
- `create <name>` - Scaffold a new lane repo (`--output`, `--description`, `--yes`)
- `validate [path]` - Validate lane definitions in fledge.toml (`--strict`, `--json`)

**Shortcut:** `fledge lanes ci` is equivalent to `fledge lanes run ci`.

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
fledge lanes run ci           # run a lane
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

### fledge watch

Re-run a task automatically when files change.

```text
fledge watch [task] [OPTIONS]
```

**Arguments:**
- `[task]` - Task to re-run on changes [default: the last `fledge run` task]

**Options:**
- `--debounce <MS>` - Debounce interval in milliseconds [default: `500`]

Watches the project directory for file changes and re-runs the specified task. Ignores `.git/`, `target/`, `node_modules/`, and other common build directories.

```bash
fledge watch test            # re-run tests on save
fledge watch build           # rebuild on change
```

---

## Spec: Spec-sync

### fledge spec `<action>`

Spec-sync management. Specs are the source of truth for module design.

```text
fledge spec <check|init|new|list|show> [OPTIONS]
```

**Subcommands:**

- `check` - Validate specs against code (`--strict` for warnings as errors, `--json` for machine-readable output)
- `init` - Set up spec-sync for the project
- `new <name>` - Scaffold a new spec
- `list` - List all specs (`--json` for machine-readable output)
- `show <name>` - Show a specific spec's content (`--json` for machine-readable output)

```bash
fledge spec check
fledge spec check --strict
fledge spec check --json
fledge spec init
fledge spec new auth
fledge spec list
fledge spec list --json
fledge spec show plugin
fledge spec show plugin --json
```

---

## AI: Ask and Review

### fledge ai `<action>`

Manage AI provider and model selection, the daily-driver way to switch between Claude and any Ollama-speaking endpoint.

```text
fledge ai <status|models|use> [OPTIONS]
```

**Subcommands:**

- `status [--json]`: Show active provider, model, and host with a `(from env / config / default)` source tag on each value
- `models --provider {claude,ollama} [--search <q>] [--json]`: Live list of available models (Ollama hits `/api/tags`; Claude returns curated aliases)
- `use [provider] [model]`: Interactive picker (live model list for Ollama) or fully scriptable via positional args. Writes to `~/.config/fledge/config.toml`

```bash
fledge ai status                                  # who's active and why
fledge ai models --provider ollama --search cloud
fledge ai use                                     # interactive picker
fledge ai use ollama llama3.2:latest              # scriptable
```

---

### fledge review

AI code review. Single-model by default; pass `--with-model` to run a multi-model panel in parallel against the same diff and spec context.

```text
fledge review [OPTIONS]
```

**Options:**
- `-b, --base <BRANCH>`: Base branch [default: auto-detect]
- `-f, --file <FILE>`: Review a single file
- `-m, --model <MODEL>`: Override the active provider's model
- `--provider {claude,ollama}`: Override the active provider
- `-p, --prompt <TEXT>`: Append a custom focus prompt
- `--format {summary,checklist,inline}`: Output format [default: summary]
- `--with-specs <NAMES>`: Force-include specs (comma-separated, repeatable)
- `--no-auto-specs`: Skip auto-detection of relevant specs
- `--with-model <REF>`: Add another model to the review panel (repeatable, comma-separated). Format: `provider[:model]`
- `--no-active`: Drop the active config from the panel; only run explicit `--with-model` entries
- `--json`: JSON output (single-model: legacy fields + `reviews[]`; multi-model: `reviews[]` only)

**Default branch detection:** When `--base` is not specified, fledge tries `git symbolic-ref refs/remotes/origin/HEAD`, then checks for `main` and `master` branches. Falls back to `main` if none exist.

```bash
fledge review                                                 # active model
fledge review --with-model ollama                             # active + 1 more
fledge review --no-active --with-model claude:sonnet,ollama   # exactly two models, no active
fledge review --json | jq '.reviews[].provider'
```

---

### fledge ask `<question>`

Ask about your codebase. Spec-aware by default, the active model gets a compact index of every spec injected into the prompt.

```text
fledge ask <question> [OPTIONS]
```

**Options:**
- `-m, --model <MODEL>`: Override active model
- `--provider {claude,ollama}`: Override active provider
- `--with-specs <NAMES>`: Include full spec + companions for these modules (comma-separated; pass `all` for everything)
- `--no-spec-index`: Skip the spec-index injection (for off-topic questions)
- `--json`: JSON output

```bash
fledge ask "how does the template rendering work?"
fledge ask --with-specs work,trust "how do these modules interact?"
fledge ask --with-specs all "which modules touch GitHub?"
```

---

## Ship: Branch, Commit, Push, Release

### fledge work `<action>`

Git workflow for feature branches. Supports any branch type, not just features. PR creation uses `gh pr create` or the GitHub web UI.

```text
fledge work <start|commit|push|status> [OPTIONS]
```

**Subcommands:**

- `start <name>`: Create a work branch (`--json` for JSON output)
- `commit`: Stage and commit with conventional-commit formatting (`--json` for JSON output)
- `push`: Push the current branch to origin (`--json` for JSON output)
- `status`: Current branch status — ahead/behind counts and dirty file count (`--json` for JSON output)

**Options for `work start`:**

- `-t, --branch-type <TYPE>`: Branch type: `feat`, `feature`, `fix`, `bug`, `chore`, `task`, `docs`, `hotfix`, `refactor` [default: `feat`]
- `-i, --issue <NUMBER>`: Link to GitHub issue (prefixes branch name with issue number)
- `--prefix <PREFIX>`: Override branch prefix entirely (e.g. `user/leif`)
- `--base <BRANCH>`: Base branch [default: `main`]

**Options for `work commit`:**

- `-m, --message <MSG>`: Commit message (prompted interactively if omitted)
- `-t, --type <TYPE>`: Commit type: `feat`, `fix`, `chore`, `docs`, `refactor`, etc. (default: inferred from branch prefix)
- `-s, --scope <SCOPE>`: Scope for conventional commit (e.g. `work`, `cli`)
- `-a, --all`: Stage all changes (including untracked) before committing
- `--ai`: Generate the commit message via the configured LLM from the staged diff
- `--provider {claude,ollama}`: Override AI provider for `--ai`
- `--model <MODEL>`: Override AI model for `--ai`
- `--json`: Emit `{schema_version, action, hash, message, branch}`

**Options for `work push`:**

- `-f, --force`: Force push with `--force-with-lease` for safety
- `--json`: Emit `{schema_version, action, branch, remote, force}`

The branch format is configurable via `[work]` in `fledge.toml`:

```toml
[work]
default_type = "feat"
branch_format = "{author}/{type}/{name}"
```

**Examples:**

```bash
fledge work start add-auth                            # leif/feat/add-auth
fledge work start login-crash --branch-type fix       # leif/fix/login-crash
fledge work start bump-deps --branch-type chore       # leif/chore/bump-deps
fledge work start login-crash --issue 42              # leif/feat/42-login-crash
fledge work start my-feature --prefix user/leif       # user/leif/my-feature
fledge work commit -m "add search index"              # explicit message
fledge work commit --ai --all                         # AI message + stage everything
fledge work push                                      # push to origin
fledge work push --force                              # force-with-lease
fledge work status --json                             # {schema_version, action, branch, default, ahead, behind, dirty}
```

---

### fledge changelog

Generate a changelog from git tags and conventional commits.

```text
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

Cut a release. Bump version, generate changelog, create a git tag, and optionally push.

```text
fledge release <bump> [OPTIONS]
```

**Arguments:**
- `<bump>`: Version bump: `major`, `minor`, `patch`, or an explicit version (e.g. `1.0.0`)

**Options:**
- `--dry-run`: Show what would happen without making changes
- `--no-tag`: Skip creating a git tag
- `--no-changelog`: Skip changelog generation
- `--no-bump`: Skip bumping any version files (tag-only release)
- `--push`: Push commit and tag to remote after release
- `--pre-lane <NAME>`: Run a lane before releasing (e.g. `ci`)
- `--allow-dirty`: Allow releasing with uncommitted changes
- `--json`: Emit a JSON envelope. Suppresses prose output

**Output (`--json --dry-run`):** `{schema_version: 1, action: "release", dry_run: true, version, no_bump, files_to_bump, will_changelog, will_tag, will_push, tag}`

**Output (`--json` real run):** `{schema_version: 1, action: "release", dry_run: false, version, old_version, files_bumped, changelog_updated, commit_created, tag_created, tag, pushed}`

**Examples:**

```bash
fledge release patch                          # bump patch version
fledge release minor --push                   # bump minor + push
fledge release major --pre-lane ci            # run CI lane first, then bump major
fledge release 2.0.0 --dry-run                # preview a specific version bump
fledge release 2.0.0 --dry-run --json         # preview as JSON
fledge release patch --no-tag --no-changelog  # just bump version
```

---

### fledge github issues `[view <number>]` (plugin)

Provided by [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github). List and view GitHub issues.

```text
fledge github issues [list] [OPTIONS]
fledge github issues view <number> [OPTIONS]
```

**Options:**
- `-s, --state <STATE>`: `open`, `closed`, `all` [default: `open`]
- `-l, --limit <N>`: Max results [default: `20`]
- `--label <LABEL>`: Filter by label
- `--json`

---

### fledge github prs `[view <number>]` (plugin)

Provided by [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github). List and view pull requests. Use `gh pr create` or the GitHub web UI to open new PRs.

```text
fledge github prs [list] [OPTIONS]
fledge github prs view <number> [OPTIONS]
```

**Options:**
- `-s, --state <STATE>`: `open`, `closed`, `merged`, `all` [default: `open`]
- `-l, --limit <N>`: Max results [default: `20`]
- `--json`

---

### fledge github checks (plugin)

Provided by [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github). CI/CD status for a branch.

```text
fledge github checks [OPTIONS]
```

**Options:**
- `-b, --branch <BRANCH>`: Branch to check [default: current]
- `--json`

---

## Extend: Plugins, Config, Tools

### fledge plugins `<action>`

Install, manage, and run community plugins.

```text
fledge plugins <install|remove|update|list|search|run|publish|create|validate|audit> [OPTIONS]
```

**Subcommands:**

- `install <source[@ref]> | --defaults`: Install from GitHub (`owner/repo[@tag]` or URL). `--force` to reinstall. Use `@ref` to pin to a tag, branch, or commit. **`--defaults`** installs the curated plugin set (`fledge-plugin-{github,deps,metrics}`) in one shot.
- `remove <name>`: Uninstall a plugin
- `update [name]`: Update plugins. Unpinned plugins get `git pull`; pinned plugins check for newer tags.
- `list`: Show installed plugins (includes pinned version info)
- `search [query]`: Find plugins on GitHub (`--author`, `--limit`)
- `run <name> [args...]`: Run a plugin command
- `publish [path]`: Publish a plugin to GitHub (`--org`, `--private`, `--description`)
- `create <name>`: Scaffold a new plugin (`--output`, `--description`, `--yes`)
- `validate [path]`: Validate a plugin manifest (`--strict`, `--json`)
- `audit`: Security audit of installed plugins (trust tier, source, capabilities)

`--json` works with `list`, `search`, and `audit`.

**Default plugins** (installed by `--defaults`):

| Repo | Adds |
|------|------|
| [`CorvidLabs/fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) | `github checks`, `github issues`, `github prs` |
| [`CorvidLabs/fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps) | `deps` |
| [`CorvidLabs/fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics) | `metrics` |

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

### fledge config `<action>`

Manage `~/.config/fledge/config.toml`.

```text
fledge config <get|set|unset|add|remove|edit|list|path|init>
```

| Subcommand | What it does |
|------------|-------------|
| `get <key>` | Read a value |
| `set <key> <value>` | Write a value |
| `unset <key>` | Delete a value |
| `add <key> <value>` | Append to a list (`templates.paths`, `templates.repos`) |
| `remove <key> <value>` | Remove from a list |
| `edit` | Interactively browse and edit config values (requires TTY) |
| `list` | Show everything |
| `path` | Print config file path |
| `init [--preset <name>]` | Initialize config (presets: `corvidlabs`) |

**Valid keys:**
- `defaults.author`, `defaults.github_org`, `defaults.license`
- `github.token`
- `templates.paths`, `templates.repos`
- `ai.provider`, `ai.claude.model`
- `ai.ollama.host`, `ai.ollama.api_key`, `ai.ollama.model`, `ai.ollama.timeout_seconds`

```bash
fledge config set defaults.author "Leif"
fledge config add templates.repos "CorvidLabs/fledge-templates"
fledge config edit                                  # interactive config editor
fledge config list
```

---

### fledge doctor

Diagnose fledge's environment health. Reports four sections:

- **`fledge`**: config loads cleanly
- **`Git`**: git installed; repo initialized; remote configured; working tree clean
- **`AI`**: Claude CLI present, Ollama reachable, the active provider's status
- **`Toolchains`** *(informational)*: probes 16 toolchains across rust (`rustc`, `cargo`), node (`node`, `npm`, `pnpm`, `bun`, `yarn`), python (`python3`, `uv`, `poetry`), `go`, `ruby`, `swift`, JVM (`java`, `gradle`, `mvn`). Missing entries render dimmed (`· tool (not installed)`) and don't pollute the pass/fail totals. A Python project shouldn't fail because Swift is absent.

```text
fledge doctor [OPTIONS]
```

**Options:**
- `--json` - JSON output

**Output (`--json`):** `{sections: [{name, checks: [{name, status, version, detail, fix}], informational}], passed, failed}`. Informational sections (e.g. `Toolchains`) appear in the JSON with `informational: true` and are excluded from the `passed`/`failed` totals.

---

### fledge introspect

Dump the full command tree as JSON. Includes core commands and any installed plugin commands. Designed for AI agents and automation.

```text
fledge introspect [OPTIONS]
```

**Options:**
- `--json` - JSON output (default behavior; flag exists for consistency)

**Output:** `{schema_version: 1, name, about, aliases, args, subcommands}` recursively. Each subcommand has the same shape, so the tree is fully walkable.

```bash
fledge introspect --json
fledge introspect --json | jq '.subcommands[].name'
```

---

### fledge completions `[shell]`

Shell completions for bash, zsh, fish.

```text
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

### fledge deps (plugin)

Provided by [`fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps). Auto-detects ecosystem from lockfiles and shells out to the canonical tool.

```text
fledge deps [--outdated | --audit | --licenses] [--json]
```

| Lockfile | Ecosystem | Backing tool |
|----------|-----------|--------------|
| `Cargo.lock` | Rust | `cargo outdated` / `cargo audit` |
| `bun.lockb` | Node (Bun) | `bun outdated` / `bun audit` |
| `pnpm-lock.yaml` | Node (pnpm) | `pnpm outdated` / `pnpm audit` |
| `package-lock.json` | Node (npm) | `npm outdated` / `npm audit` |
| `yarn.lock` | Node (Yarn) | `yarn outdated` / `yarn npm audit` |
| `poetry.lock` | Python (Poetry) | `poetry show --outdated` |
| `uv.lock` | Python (uv) | `uv pip list --outdated` |

---

### fledge metrics (plugin)

Provided by [`fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics). Thin wrapper over `tokei` (LOC) and `git` (churn).

```text
fledge metrics [--churn | --tests] [-l N] [--json]
```

```bash
fledge metrics                       # LOC summary by language (tokei)
fledge metrics --churn -l 10         # top-10 most-changed files
fledge metrics --tests --json        # {test_files, source_files, ratio}
```

---
