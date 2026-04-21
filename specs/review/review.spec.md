---
module: review
version: 2
status: active
files:
  - src/review.rs

db_tables: []
depends_on: []
---

# Review

## Purpose

AI-powered code review of current branch changes. Gets the git diff against a base branch and sends it to the Claude CLI for review, displaying actionable feedback inline.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point for the review command |
| `ReviewOptions` | Options struct with base branch, file filter, and json flag |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ReviewOptions` | `{ base: Option<String>, file: Option<String>, json: bool }` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(ReviewOptions) -> Result<()>` | Runs AI code review on current diff |

## Invariants

1. Requires Claude CLI (`claude`) to be installed and authenticated
2. Base branch defaults to auto-detected default (main/master)
3. Empty diffs bail with a clear message
4. Shows diff stats before the AI review output
5. `--file` flag restricts review to a single file's changes
6. `--json` outputs structured JSON review results

## Behavioral Examples

### review — review all changes
```
$ fledge review
● Reviewing changes against main ...

 src/github.rs | 85 ++++++++++++
 src/issues.rs | 120 +++++++++++++++
 2 files changed, 205 insertions(+)

[AI review output streamed here]
```

### review — against specific base
```
$ fledge review --base develop
● Reviewing changes against develop ...
```

### review — single file
```
$ fledge review --file src/github.rs
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Claude CLI not installed | `claude --version` fails | Bail with install instructions |
| Not a git repo | Outside a git repository | Bail with message |
| No changes | Empty diff against base | Bail with message |
| Claude CLI error | Non-zero exit from claude | Bail with error |

## Dependencies

- Claude CLI — AI inference (external dependency)
- Git CLI — diff generation

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2 | 2026-04-21 | Add json field to ReviewOptions |
| 1 | 2026-04-19 | Initial spec |
