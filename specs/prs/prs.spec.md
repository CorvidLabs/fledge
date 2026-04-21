---
module: prs
version: 2
status: active
files:
  - src/prs.rs

db_tables: []
depends_on:
  - github
  - config
---

# Prs

## Purpose

Lists and views GitHub pull requests for the current repository. Provides `fledge prs` (list) and `fledge prs view <number>` (detail view) with state filtering, draft detection, and diff stats.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to list or view |
| `PrsAction` | Enum of subcommands: List, View |

### Structs & Enums

| Type | Description |
|------|-------------|
| `PrsAction` | Enum: `List { state, limit, json }` or `View { number, json }` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(PrsAction) -> Result<()>` | Dispatches to list or view |

## Invariants

1. Repository is auto-detected from git remote origin
2. Draft PRs are shown with a distinct icon (open circle vs filled)
3. Merged PRs show "merged" state in magenta
4. Detail view includes diff stats (files changed, additions, deletions)
5. Default state filter is "open", sorted by recently updated

## Behavioral Examples

### prs — list open PRs
```
$ fledge prs
Open pull requests in CorvidLabs/fledge:

  🟢 #45    Add spec command           feat/spec-command ➡️ base  1h ago
  📝 #44    WIP: work command          feat/work ➡️ base          3h ago
```

### prs view — view specific PR
```
$ fledge prs view 45
#45 Add spec command  open
  Opened by corvid-agent 1h ago
  Branch: feat/spec-command → main
  Diff: 3 files changed, +150, -10
  Comments: 2

PR description here...
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No git remote | Not in a GitHub repo | Bail with message |
| 404 | PR number doesn't exist | Bail with "Not found" |
| Rate limit | No token and rate limited | Bail with token instructions |

## Dependencies

- `github` — repo detection and API calls
- `config` — GitHub token
- `console` — styled output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 1 | 2026-04-19 | Initial spec |
