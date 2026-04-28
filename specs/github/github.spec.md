---
module: github
version: 2
status: active
files:
  - src/github.rs

db_tables: []
depends_on: []
---

# GitHub

## Purpose

Shared helpers for GitHub API interactions: authenticated REST API calls and environment-readiness probes. Used by `work pr`, `release`, and the AI ask/review path (for `claude` CLI presence).

In v0.15 this module shrank from a generic GitHub client into a small set of "hard prerequisites" — repo detection and relative-time formatting moved out with the deleted `checks`/`issues`/`prs` commands (they live in `fledge-plugin-github` now).

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `github_api_get` | Makes an authenticated GET request to the GitHub REST API |
| `ensure_claude_cli` | Verifies that the Claude CLI is installed and accessible |
| `ensure_git_repo` | Verifies that the current directory is inside a git repository |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `github_api_get` | `(path, token, query_params) -> Result<Value>` | GET request to GitHub API with optional auth |
| `ensure_claude_cli` | `() -> Result<()>` | Checks `claude --version` succeeds, bails if not installed |
| `ensure_git_repo` | `() -> Result<()>` | Runs `git rev-parse --is-inside-work-tree`, bails if not a repo |

## Invariants

1. `github_api_get` accepts an `Option<&str>` token; when `None`, the request is unauthenticated and subject to GitHub's anon rate limits
2. Rate limit errors (403) produce a helpful message about setting a token via `fledge config set github.token`
3. 404 errors include the resolved repo identifier when extractable from the path so users can spot a typo or private-repo issue
4. `ensure_claude_cli` probes the `claude` binary via `--version`; failure bails with the install URL
5. `ensure_git_repo` uses `git rev-parse --is-inside-work-tree`; non-repo dirs bail with "Not a git repository"

## Behavioral Examples

### github_api_get — authenticated request
```
github_api_get("/repos/CorvidLabs/fledge/issues", Some(token), &[("state", "open")])
  -> GET https://api.github.com/repos/CorvidLabs/fledge/issues?state=open
  -> Authorization: Bearer <token>
```

### ensure_claude_cli — missing binary
```
$ ensure_claude_cli()
Err: Claude CLI is not installed. Install it from
     https://docs.anthropic.com/en/docs/claude-code and run `claude` to authenticate.
```

### ensure_git_repo — outside a repo
```
$ cd /tmp && ensure_git_repo()
Err: Not a git repository.
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| 404 | Resource not found | Bail with "Not found" + repo id + token hint |
| 403 | Rate limit exceeded | Bail with token setup instructions |
| Claude CLI missing | `ensure_claude_cli` when `claude` not on PATH | Bail with install URL |
| Not a git repo | `ensure_git_repo` outside a git worktree | Bail with "Not a git repository" |

## Dependencies

- `ureq` — HTTP client
- `search::urlencod` — URL parameter encoding for query strings

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2 | 2026-04-25 | v0.15 tight-core: remove `detect_repo`, `parse_repo_url`, `format_relative_time`, they only existed for the deleted `checks`/`issues`/`prs` commands and now live in `fledge-plugin-github`. `parse_repo_url` retained as a `#[cfg(test)]` helper. |
| 1 | 2026-04-21 | Add ensure_git_repo and ensure_claude_cli exports |
| 1 | 2026-04-19 | Initial spec |
