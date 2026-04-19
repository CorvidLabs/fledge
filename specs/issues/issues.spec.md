---
module: issues
version: 1
status: active
files:
  - src/issues.rs

db_tables: []
depends_on:
  - specs/github/github.spec.md
  - specs/config/config.spec.md
---

# Issues

## Purpose

Lists and views GitHub issues for the current repository. Provides `fledge issues` (list) and `fledge issues view <number>` (detail view) commands with filtering, label support, and JSON output.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to list or view |
| `IssuesAction` | Enum of subcommands: List, View |

### Structs & Enums

| Type | Description |
|------|-------------|
| `IssuesAction` | Enum: `List { state, limit, json, label }` or `View { number, json }` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(IssuesAction) -> Result<()>` | Dispatches to list or view |

## Invariants

1. Repository is auto-detected from git remote origin
2. Pull requests are filtered out of issue listings (GitHub API returns both)
3. Default state filter is "open", sorted by recently updated
4. `view` detects if a number is a PR and suggests `fledge prs` instead
5. JSON output returns the raw GitHub API response

## Behavioral Examples

### issues — list open issues
```
$ fledge issues
Open issues in CorvidLabs/fledge:

  ● #36    Lanes & plugin system             enhancement  0d ago
  ● #35    AI-powered code assistance         enhancement  0d ago
  ● #34    GitHub ops from CLI                enhancement  0d ago
```

### issues view — view specific issue
```
$ fledge issues view 34
#34 GitHub ops from CLI  open
  Opened by leif-algo 2d ago
  Labels: enhancement
  Comments: 3

Issue body text here...
```

### issues — with label filter
```
$ fledge issues --label bug
```

### issues — JSON output
```
$ fledge issues --json
[{ ... }]
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No git remote | Not in a GitHub repo | Bail with message |
| 404 | Issue number doesn't exist | Bail with "Not found" |
| PR number given | `view` with a PR number | Bail suggesting `fledge prs` |
| Rate limit | No token and rate limited | Bail with token instructions |

## Dependencies

- `github` — repo detection and API calls
- `config` — GitHub token
- `console` — styled output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec |
