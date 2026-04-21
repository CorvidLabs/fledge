---
module: checks
version: 2
status: active
files:
  - src/checks.rs

db_tables: []
depends_on:
  - github
  - config
---

# Checks

## Purpose

View CI/CD check run status for a branch using the GitHub Check Runs API. Shows pass/fail/pending status with timing information.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — fetch and display check status |
| `ChecksOptions` | Options: `branch`, `json` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ChecksOptions` | Options: `branch`, `json` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(ChecksOptions) -> Result<()>` | Fetch checks from GitHub API and display |

## Invariants

1. Defaults to current branch if `--branch` is not specified
2. Uses GitHub token from config if available
3. Displays check name, status, and duration for each check run
4. Shows summary counts (passed/failed/pending)
5. Supports `--json` for raw API output

## Behavioral Examples

```
$ fledge checks
* CI checks for feat/v0.7.0:

  ✅ lint          passed      12s
  ✅ test-ubuntu   passed      1m 30s
  ❌ test-windows  failed      45s
  🔄 audit         running     running...

  3 checks: 2 passed, 1 failed, 1 pending

$ fledge checks --branch main --json
{ ... }
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Not a git repo | No origin remote | Error from github::detect_repo |
| Detached HEAD | No branch and no --branch flag | Error with suggestion |
| API rate limit | No token and rate limited | Error with config guidance |
| Branch not found | No commits for branch | Empty checks list |

## Dependencies

- `github` — repo detection and API calls
- `config` — GitHub token

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 1 | 2026-04-19 | Initial spec |
