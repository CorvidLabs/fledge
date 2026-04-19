---
module: github
version: 1
status: active
files:
  - src/github.rs

db_tables: []
depends_on: []
---

# GitHub

## Purpose

Shared helpers for GitHub API interactions: repository detection from git remotes, authenticated REST API calls, and time formatting. Used by `issues`, `prs`, and other modules that need GitHub data.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `detect_repo` | Detects GitHub owner/repo from the current git remote |
| `github_api_get` | Makes an authenticated GET request to the GitHub REST API |
| `format_relative_time` | Formats an ISO 8601 timestamp as a human-readable relative time |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `detect_repo` | `() -> Result<(String, String)>` | Parses origin remote URL to extract owner and repo |
| `github_api_get` | `(path, token, query_params) -> Result<Value>` | GET request to GitHub API with optional auth |
| `format_relative_time` | `(iso: &str) -> String` | Converts ISO timestamp to "5m ago", "3h ago", etc. |

## Invariants

1. `detect_repo` supports both HTTPS and SSH remote URLs, with or without `.git` suffix
2. `detect_repo` supports token-authenticated HTTPS URLs (e.g., `https://token@github.com/...`)
3. `github_api_get` uses the `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN` env vars, or config token
4. Rate limit errors (403) produce a helpful message about setting a token
5. `format_relative_time` gracefully falls back to the raw string for unparseable input

## Behavioral Examples

### detect_repo — HTTPS remote
```
# Given remote: https://github.com/CorvidLabs/fledge.git
detect_repo() -> Ok(("CorvidLabs", "fledge"))
```

### detect_repo — SSH remote
```
# Given remote: git@github.com:CorvidLabs/fledge.git
detect_repo() -> Ok(("CorvidLabs", "fledge"))
```

### detect_repo — token-authenticated URL
```
# Given remote: https://ghp_abc@github.com/CorvidLabs/fledge.git
detect_repo() -> Ok(("CorvidLabs", "fledge"))
```

### github_api_get — authenticated request
```
github_api_get("/repos/CorvidLabs/fledge/issues", Some(token), &[("state", "open")])
  -> GET https://api.github.com/repos/CorvidLabs/fledge/issues?state=open
  -> Authorization: Bearer <token>
```

### format_relative_time
```
format_relative_time("2026-04-19T10:00:00Z")  // now is 10:05
  -> "5m ago"

format_relative_time("not-a-date")
  -> "not-a-date"
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No origin remote | `detect_repo` outside a repo or without origin | Bail with message |
| Unparseable URL | Remote URL is not a GitHub URL | Bail with the URL shown |
| 404 | Resource not found | Bail with "Not found" |
| 403 | Rate limit exceeded | Bail with token setup instructions |

## Dependencies

- `ureq` — HTTP client
- `chrono` — time parsing and relative formatting
- `search::urlencod` — URL parameter encoding

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec |
