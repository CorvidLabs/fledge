---
module: work
version: 5
status: active
files:
  - src/work.rs

db_tables: []
depends_on:
  - config
---

# Work

## Purpose

Provides opinionated git workflow commands for feature branch development. `fledge work start` creates a branch following configurable naming conventions (type, issue linking, custom prefix), and `fledge work pr` creates a pull request from the current branch with automatic title/body generation.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate work subcommand |
| `WorkAction` | Enum of subcommands: Start, Pr, Status |
| `sanitize_branch_name` | Normalizes a string into a valid git branch name (lowercase, hyphens, no leading/trailing hyphens) |
| `generate_title_from_branch` | Generates a human-readable PR title from a branch name by stripping any known type prefix and converting hyphens to spaces |
| `build_branch_name` | (test-only) Constructs a branch name from components using WorkConfig |

### Structs & Enums

| Type | Description |
|------|-------------|
| `WorkAction` | Enum of subcommands: Start (name, branch_type, issue, prefix, base), Pr, Status |
| `WorkConfig` | Deserializable config with `branch_format` and `default_type` fields |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(WorkAction) -> Result<()>` | Dispatches to start, pr, or status |
| `start` | `(name, branch_type, issue, prefix, base) -> Result<()>` | Creates and checks out a branch with configurable type and naming |
| `pr` | `(title, body, draft, base) -> Result<()>` | Creates a PR via `gh` CLI |
| `status` | `() -> Result<()>` | Shows current branch, commits ahead, and PR status |
| `sanitize_branch_name` | `(&str) -> String` | Lowercase, replace special chars with hyphens, collapse consecutive hyphens |
| `generate_title_from_branch` | `(&str) -> String` | Strip type prefix, convert hyphens to spaces, title-case |
| `build_branch_name` | `(name, branch_type, issue, prefix, config) -> String` | Apply format template with `{author}`, `{type}`, `{name}`, `{issue}` substitution |

**Internal (not exported):**
- `load_work_config() -> WorkConfig` — Reads `[work]` section from `fledge.toml`, falls back to defaults

## Invariants

1. Branch names are normalized via `sanitize_branch_name`: lowercase, special chars become hyphens, no consecutive or trailing hyphens
2. Default branch format is `{author}/{type}/{name}` — configurable via `[work]` in `fledge.toml`
3. Valid branch types: `feat`, `feature`, `fix`, `bug`, `chore`, `task`, `docs`, `hotfix`, `refactor` (enforced unless `--prefix` is used)
4. Default branch type is `feat` — configurable via `[work] default_type` in `fledge.toml`
5. `{author}` resolves from global config `defaults.author` or `git config user.name`
6. `start` refuses to create a branch if there are uncommitted changes
7. `pr` requires `gh` CLI to be installed and authenticated
8. `pr` pushes the current branch to origin before creating the PR
9. `status` works without `gh` (gracefully degrades if not available)
10. `--prefix` bypasses type validation and format template, using raw `prefix/name`
11. `--issue N` prepends the issue number to the branch name segment: `N-name`
12. `generate_title_from_branch` strips any valid branch type prefix (feat/, feature/, fix/, bug/, chore/, task/, docs/, hotfix/, refactor/)
13. Plugin lifecycle hook `post_work_start` runs after branch creation (errors silently ignored via `.ok()`)
14. Plugin lifecycle hook `pre_pr` runs before PR creation (errors propagate and abort the PR)

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

### work pr — create pull request
```
$ fledge work pr
✓ Pushed leif/feat/add-search to origin
✓ Created PR #42: "Add search command"
  https://github.com/owner/repo/pull/42
```

### work pr — with title and draft
```
$ fledge work pr --title "WIP: search command" --draft
✓ Pushed leif/feat/add-search to origin
✓ Created draft PR #42: "WIP: search command"
  https://github.com/owner/repo/pull/42
```

### work status — on feature branch
```
$ fledge work status
  Branch: leif/feat/add-search (3 commits ahead of main)
  PR: #42 (open) — https://github.com/owner/repo/pull/42
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Not a git repository | Any subcommand outside git repo | Bail with message |
| Uncommitted changes | `work start` with dirty tree | Bail with message |
| Branch already exists | `work start` with existing branch name | Bail with message |
| Unknown branch type | `work start --branch-type <invalid>` without `--prefix` | Bail with valid types list |
| On main/master | `work pr` from default branch | Bail with message |
| `gh` not installed | `work pr` | Bail with install instructions |
| No commits ahead | `work pr` with no new commits | Bail with message |

## Dependencies

- `console` — styled terminal output
- `serde` / `toml` — config deserialization for `[work]` section in `fledge.toml`
- `crate::config::Config` — global config for author resolution
- Git CLI — branch operations
- `gh` CLI — PR creation (optional for `status`)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec for fledge work |
| 2 | 2026-04-20 | Flexible branch types, configurable format, `{author}` support, `--type`/`--issue`/`--prefix` flags |
| 5 | 2026-04-22 | Document lifecycle hooks: post_work_start (silent) and pre_pr (propagating) |
| 4 | 2026-04-21 | Correct load_work_config as internal (not exported) |
| 3 | 2026-04-20 | Added `feature`, `bug`, `task` as valid branch types |
