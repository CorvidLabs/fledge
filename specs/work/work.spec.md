---
module: work
version: 13
status: active
files:
  - src/work.rs

db_tables: []
depends_on:
  - config
  - llm
---

# Work

## Purpose

Provides opinionated git workflow commands for feature branch development. `fledge work start` creates a branch following configurable naming conventions (type, issue linking, custom prefix), `fledge work commit` creates conventional-commit formatted commits with optional AI message generation, and `fledge work push` pushes the current branch to origin with safety guards. PR creation has been extracted to `fledge-plugin-github`; `fledge work pr` prints a deprecation notice directing users there.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate work subcommand |
| `WorkAction` | Enum of subcommands: Start, Commit, Push, Status, DeprecatedPr |
| `sanitize_branch_name` | Normalizes a string into a valid git branch name (lowercase, hyphens, no leading/trailing hyphens) |
| `generate_title_from_branch` | Generates a human-readable PR title from a branch name by stripping any known type prefix and converting hyphens to spaces (retained for plugin use, `#[allow(dead_code)]`) |
| `build_commit_message` | Builds a conventional-commit message string from type, optional scope, and message body |
| `build_branch_name` | (test-only) Constructs a branch name from components using WorkConfig |

### Structs & Enums

| Type | Description |
|------|-------------|
| `WorkAction` | Enum of subcommands: Start (name, branch_type, issue, prefix, base, json), Commit (message, commit_type, scope, all, ai, provider, model, json), Push (force, json), Status (json), DeprecatedPr |
| `WorkConfig` | Deserializable config with `branch_format` and `default_type` fields |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(WorkAction) -> Result<()>` | Dispatches to start, commit, push, status, or deprecated-pr notice |
| `start` | `(name, branch_type, issue, prefix, base, json) -> Result<()>` | Creates and checks out a branch with configurable type and naming |
| `commit` | `(message, commit_type, scope, all, ai, provider, model, json) -> Result<()>` | Stages (with `--all`), builds a conventional-commit message, and runs `git commit`. Infers type from branch prefix or config default |
| `push` | `(force, json) -> Result<()>` | Pushes current branch to origin with `-u`. `--force` uses `--force-with-lease`. Refuses to push default branch or when nothing to push |
| `status` | `(json: bool) -> Result<()>` | Shows current branch, commits ahead/behind, and dirty file count |
| `sanitize_branch_name` | `(&str) -> String` | Lowercase, replace special chars with hyphens, collapse consecutive hyphens |
| `generate_title_from_branch` | `(&str) -> String` | Strip type prefix, convert hyphens to spaces, title-case |
| `build_commit_message` | `(commit_type, scope, message) -> String` | Formats `type(scope): message` or `type: message`; lowercases the first character of the message |
| `build_branch_name` | `(name, branch_type, issue, prefix, config) -> String` | Apply format template with `{author}`, `{type}`, `{name}`, `{issue}` substitution |

**Internal (not exported):**
- `load_work_config() -> WorkConfig` — Reads `[work]` section from `fledge.toml`, falls back to defaults
- `generate_commit_message_with_ai(commit_type, scope, provider, model, json) -> Result<String>` — Sends the staged diff (truncated to 400 lines) to the configured LLM to generate a conventional-commit message
- `format_commit_subject_as_bullet(&str) -> String` — Strips a leading conventional-commit prefix and upper-cases the first letter

## Invariants

1. Branch names are normalized via `sanitize_branch_name`: lowercase, special chars become hyphens, no consecutive or trailing hyphens
2. Default branch format is `{author}/{type}/{name}` — configurable via `[work]` in `fledge.toml`
3. Valid branch types: `feat`, `feature`, `fix`, `bug`, `chore`, `task`, `docs`, `hotfix`, `refactor` (enforced unless `--prefix` is used)
4. Default branch type is `feat` — configurable via `[work] default_type` in `fledge.toml`
5. `{author}` resolves from global config `defaults.author` or `git config user.name`
6. `start` refuses to create a branch if there are uncommitted changes
7. Plugin lifecycle hook `post_work_start` runs after branch creation (errors silently ignored via `.ok()`)
8. Plugin lifecycle hook `pre_push` runs before `fledge work push` pushes to origin (errors propagate and abort the push)
9. `--prefix` bypasses type validation and format template, using raw `prefix/name`
10. `--issue N` prepends the issue number to the branch name segment: `N-name`
11. `generate_title_from_branch` strips any valid branch type prefix (feat/, feature/, fix/, bug/, chore/, task/, docs/, hotfix/, refactor/)
12. `commit` infers the commit type from the current branch prefix (e.g. `feat/` → `feat`) when `--type` is not provided; falls back to `WorkConfig.default_type`
13. `commit --all` runs `git add -A` before committing
14. `commit` requires staged changes; bails if nothing is staged (separate message if working tree is clean vs unstaged)
15. `commit` without `-m` or `--ai` prompts interactively via `dialoguer::Input`; non-interactive shells must provide `-m` or `--ai`
16. `commit --ai` sends the staged diff (truncated to 400 lines) to the configured LLM to generate the message; `--provider` / `--model` override for this single call
17. `push` refuses to push the default branch (main/master)
18. `push` checks commits ahead of `origin/<branch>` and refuses when there is nothing to push
19. `push` uses `--force-with-lease` (not `--force`) when the `--force` flag is passed
20. `push` always sets `-u origin` to establish tracking
21. `work pr` prints a deprecation notice directing users to `gh pr create` and exits with code 1
22. `--json` on `start` emits `{schema_version, action, branch, base, type, prefix, issue}` and suppresses the pretty output
23. `--json` on `commit` emits `{schema_version, action, hash, message, branch}` and suppresses the pretty output
24. `--json` on `push` emits `{schema_version, action, branch, remote, force}` and suppresses the spinner + pretty output
25. `--json` on `status` emits `{schema_version, action, branch, default, ahead, behind, dirty}`. `behind` is `null` when `git rev-list` can't compute it. `dirty` is the count of files with uncommitted changes
26. `--json` never silences errors — error messages still go to stderr; exit code is still non-zero on failure
27. Status no longer reports PR info — that responsibility moved to the GitHub plugin

## Behavioral Examples

### work start — create branch with default type
```
$ fledge work start add-search
✓ Created branch leif/feat/add-search from main
✓ Switched to leif/feat/add-search
```

### work start — with explicit type
```
$ fledge work start login-crash --branch-type fix
✓ Created branch leif/fix/login-crash from main
✓ Switched to leif/fix/login-crash
```

### work start — with issue number
```
$ fledge work start login-crash --branch-type fix --issue 42
✓ Created branch leif/fix/42-login-crash from main
✓ Switched to leif/fix/42-login-crash
```

### work start — with custom prefix (bypasses format)
```
$ fledge work start experiment --prefix sandbox
✓ Created branch sandbox/experiment from main
✓ Switched to sandbox/experiment
```

### work start — with custom base
```
$ fledge work start fix-bug --base develop
✓ Created branch leif/feat/fix-bug from develop
✓ Switched to leif/feat/fix-bug
```

### work start — dirty working tree
```
$ fledge work start new-feature
error: uncommitted changes detected. Commit or stash before starting work.
```

### work start — invalid branch type
```
$ fledge work start foo --type yolo
error: Unknown branch type 'yolo'. Valid types: feat, feature, fix, bug, chore, task, docs, hotfix, refactor
```

### work commit — with message
```
$ fledge work commit -m "add search index"
✅ Committed a1b2c3d on leif/feat/add-search
  feat: add search index
```

### work commit — with all flag
```
$ fledge work commit --all -m "wire up search"
✅ Committed d4e5f6a on leif/feat/add-search
  feat: wire up search
```

### work commit — AI-generated message
```
$ fledge work commit --ai
✅ Committed b7c8d9e on leif/feat/add-search
  feat: implement search index with fuzzy matching
```

### work commit — nothing staged
```
$ fledge work commit -m "oops"
error: No staged changes. Stage files with `git add` or use `fledge work commit --all`.
```

### work push
```
$ fledge work push
✅ Pushed leif/feat/add-search to origin
```

### work push — nothing to push
```
$ fledge work push
error: No commits ahead of 'origin/leif/feat/add-search'. Nothing to push.
```

### work push — from default branch
```
$ fledge work push
error: Refusing to push the default branch 'main'. Switch to a feature branch first.
```

### work status — on feature branch
```
$ fledge work status
  Branch: leif/feat/add-search (3 commits ahead of main)
```

### work status — with dirty files
```
$ fledge work status
  Branch: leif/feat/add-search (3 commits ahead of main)
  Dirty: 2 uncommitted files
```

### work pr — deprecated
```
$ fledge work pr
⚠ `fledge work pr` has been removed.
  Use `fledge github prs create` (fledge-plugin-github) to open a pull request.
  Or directly: `gh pr create` — https://cli.github.com/manual/gh_pr_create
```

### work start --json
```
$ fledge work start add-search --json
{
  "schema_version": 1,
  "action": "work_start",
  "branch": "leif/feat/add-search",
  "base": "main",
  "type": "feat",
  "prefix": null,
  "issue": null
}
```

### work commit --json
```
$ fledge work commit --all -m "add search" --json
{
  "schema_version": 1,
  "action": "work_commit",
  "hash": "a1b2c3d",
  "message": "feat: add search",
  "branch": "leif/feat/add-search"
}
```

### work push --json
```
$ fledge work push --json
{
  "schema_version": 1,
  "action": "work_push",
  "branch": "leif/feat/add-search",
  "remote": "origin",
  "force": false
}
```

### work status --json
```
$ fledge work status --json
{
  "schema_version": 2,
  "action": "work_status",
  "branch": "leif/feat/add-search",
  "default": "main",
  "ahead": 3,
  "behind": 0,
  "dirty": 0
}
```

### work status --json — when base hasn't been fetched
```
$ fledge work status --json
{
  "schema_version": 2,
  "action": "work_status",
  "branch": "leif/feat/add-search",
  "default": "main",
  "ahead": 3,
  "behind": null,
  "dirty": 0
}
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Not a git repository | Any subcommand outside git repo | Bail with message |
| Uncommitted changes | `work start` with dirty tree | Bail with message |
| Branch already exists | `work start` with existing branch name | Bail with message |
| Unknown branch type | `work start --branch-type <invalid>` without `--prefix` | Bail with valid types list |
| Nothing to commit (clean) | `work commit` with clean working tree | Bail: "Nothing to commit — working tree is clean." |
| Nothing staged | `work commit` with unstaged changes but nothing in index | Bail: "No staged changes. Stage files with `git add` or use `fledge work commit --all`." |
| No message in non-interactive | `work commit` without `-m` or `--ai` in non-TTY | Bail asking for `-m` or `--ai` |
| AI with no staged diff | `work commit --ai` with empty staged diff | Bail: "No staged diff for AI to analyze." |
| On default branch | `work push` from main/master | Bail with message |
| Nothing to push | `work push` with no commits ahead of tracking branch | Bail with message |
| `fledge work pr` | Any invocation | Print deprecation notice, exit 1 |
| AI generation fails | `--ai` with provider unreachable or model timeout | Bubble up the provider error verbatim |

## Dependencies

- `console` — styled terminal output
- `dialoguer` — `Input` prompt for interactive commit messages
- `serde` / `toml` — config deserialization for `[work]` section in `fledge.toml`
- `crate::config::Config` — global config for author resolution
- `crate::utils::is_interactive` — TTY gate for the interactive commit prompt
- `crate::llm::{build_provider, ProviderOverride, describe}` — `--ai` commit message generation
- `crate::spinner::Spinner` — progress indicator during AI calls and push
- `crate::github::ensure_git_repo` — validates git repository context
- Git CLI — branch and commit operations

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 12 | 2026-05-01 | **Security:** `commit --ai --scope <s>` now validates `<s>` against `[A-Za-z0-9_-]{1,64}` before interpolating it into the LLM prompt or commit message. Scopes containing whitespace, shell metacharacters, template syntax, or anything that could be read as instructions to the model are rejected at the boundary with a clear error |
| 11 | 2026-04-30 | Pure git split: removed `pr` subcommand (moved to `fledge-plugin-github`), added `commit` and `push` subcommands with `--ai` support and conventional-commit formatting. `status` drops PR info, adds `dirty` count, bumps schema to v2. `generate_body_from_commits` removed; `build_commit_message` added |
| 10 | 2026-04-26 | Doc sync, behavioral examples for `work start/pr/status --json` updated to show the post-tier-D envelope shapes (with `schema_version` and `action`). No code change |
| 9 | 2026-04-26 | Tier-D 1.0 envelope: `work start --json`, `work pr --json`, `work status --json` now include `schema_version: 1` and `action: "work_start"|"work_pr"|"work_status"` at the top level. Field shapes otherwise unchanged. Closes the gap where tier C (#274) only migrated plugins/lanes/templates and missed the cross-cutting commands |
| 8 | 2026-04-24 | `work pr --ai` generates a richer Markdown body via the configured LLM (`fledge ai use`-aware), with `--provider` / `--model` per-call overrides. Prompt includes commit log, diffstat, and truncated diff; spinner shown unless `--json` |
| 7 | 2026-04-24 | `work pr` auto-generates the PR body from commits when `--body` is omitted, shows a styled preview, and prompts for confirmation before creating the PR. `--yes` / `-y` skips the prompt; non-interactive shells must pass `--yes` or `--json` |
| 6 | 2026-04-23 | Add `--json` to `start`, `pr`, and `status`. `status` now also reports `behind`. Pretty output suppressed in JSON mode; errors still go to stderr |
| 5 | 2026-04-22 | Document lifecycle hooks: `post_work_start` (silent) and `pre_pr` (propagating) |
| 4 | 2026-04-21 | Correct `load_work_config` as internal (not exported) |
| 3 | 2026-04-20 | Add `feature`, `bug`, `task` as valid branch types |
| 2 | 2026-04-20 | Flexible branch types, configurable format, `{author}` support, `--type`/`--issue`/`--prefix` flags |
| 1 | 2026-04-19 | Initial spec for fledge work |
