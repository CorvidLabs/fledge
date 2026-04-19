---
module: work
version: 1
status: active
files:
  - src/work.rs

db_tables: []
depends_on: []
---

# Work

## Purpose

Provides opinionated git workflow commands for feature branch development. `fledge work start` creates a feature branch following naming conventions, and `fledge work pr` creates a pull request from the current branch with automatic title/body generation.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate work subcommand |

### Structs & Enums

| Type | Description |
|------|-------------|
| `WorkAction` | Enum of subcommands: Start, Pr, Status |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(WorkAction) -> Result<()>` | Dispatches to start, pr, or status |
| `start` | `(name: &str, base: Option<&str>) -> Result<()>` | Creates and checks out a feature branch |
| `pr` | `(title: Option<&str>, body: Option<&str>, draft: bool, base: Option<&str>) -> Result<()>` | Creates a PR via `gh` CLI |
| `status` | `() -> Result<()>` | Shows current branch, commits ahead, and PR status |

## Invariants

1. Branch names are normalized: spaces and special chars become hyphens, prefixed with `feat/`
2. `start` refuses to create a branch if there are uncommitted changes
3. `pr` requires `gh` CLI to be installed and authenticated
4. `pr` pushes the current branch to origin before creating the PR
5. `status` works without `gh` (gracefully degrades if not available)

## Behavioral Examples

### work start — create feature branch
```
$ fledge work start add-search
✓ Created branch feat/add-search from main
✓ Switched to feat/add-search
```

### work start — with custom base
```
$ fledge work start fix-bug --base develop
✓ Created branch feat/fix-bug from develop
✓ Switched to feat/fix-bug
```

### work start — dirty working tree
```
$ fledge work start new-feature
error: uncommitted changes detected. Commit or stash before starting work.
```

### work pr — create pull request
```
$ fledge work pr
✓ Pushed feat/add-search to origin
✓ Created PR #42: "Add search command"
  https://github.com/owner/repo/pull/42
```

### work pr — with title and draft
```
$ fledge work pr --title "WIP: search command" --draft
✓ Pushed feat/add-search to origin
✓ Created draft PR #42: "WIP: search command"
  https://github.com/owner/repo/pull/42
```

### work status — on feature branch
```
$ fledge work status
  Branch: feat/add-search (3 commits ahead of main)
  PR: #42 (open) — https://github.com/owner/repo/pull/42
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Not a git repository | Any subcommand outside git repo | Bail with message |
| Uncommitted changes | `work start` with dirty tree | Bail with message |
| Branch already exists | `work start` with existing branch name | Bail with message |
| On main/master | `work pr` from default branch | Bail with message |
| `gh` not installed | `work pr` | Bail with install instructions |
| No commits ahead | `work pr` with no new commits | Bail with message |

## Dependencies

- `console` — styled terminal output
- Git CLI — branch operations
- `gh` CLI — PR creation (optional for `status`)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec for fledge work |
