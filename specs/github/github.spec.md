---
module: github
version: 5
status: active
files:
  - src/github.rs

db_tables: []
depends_on: []
---

# GitHub

## Purpose

Shared helpers for GitHub API interactions: authenticated REST API calls and a git-repository readiness probe.

In v0.15 this module shrank from a generic GitHub client into a small set of "hard prerequisites" — repo detection and relative-time formatting moved out with the deleted `checks`/`issues`/`prs` commands (they live in `fledge-plugin-github` now).

As of v0.17, `fledge work pr` was removed from core — PR creation now lives entirely in `fledge-plugin-github`. The core `github.rs` module is no longer used by the work module.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `github_api_get` | Makes an authenticated GET request to the GitHub REST API |
| `ensure_git_repo` | Verifies that the current directory is inside a git repository |
| `api_base` | Resolves the GitHub REST base URL, redirectable in tests |
| `remote_base` / `remote_url` | Resolve the `github.com/<owner>/<repo>.git` git endpoint, redirectable in tests |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `github_api_get` | `(path, token, query_params) -> Result<Value>` | GET request to GitHub API with optional auth |
| `ensure_git_repo` | `() -> Result<()>` | Runs `git rev-parse --is-inside-work-tree`, bails if not a repo |
| `api_base` | `() -> String` | GitHub REST base; production constant, with a thread-local override under `cfg(test)` and a debug-build-only loopback env override |
| `remote_base` | `() -> String` | Git remote base (`https://github.com`), same two overrides. Single source of truth for `publish` (push) and `remote` (remote-template clone) |
| `remote_url` | `(owner, repo) -> String` | `{remote_base()}/{owner}/{repo}.git` |

## Invariants

1. `github_api_get` accepts an `Option<&str>` token; when `None`, the request is unauthenticated and subject to GitHub's anon rate limits
2. Rate limit errors (403) produce a helpful message about setting a token via `fledge config set github.token`
3. 404 errors include the resolved repo identifier when extractable from the path so users can spot a typo or private-repo issue
4. `ensure_git_repo` uses `git rev-parse --is-inside-work-tree`; non-repo dirs bail with "Not a git repository"
5. Every request URL is built on `api_base()` (crate-internal), which returns the `https://api.github.com` constant in release builds. Test builds may redirect it at a loopback mock server, so the request path — headers, query encoding, status mapping, JSON decoding — is covered without network access. `publish.rs` builds its GitHub URLs on the same helper
6. Every `github.com/<owner>/<repo>.git` git URL is built on `remote_url()`, so publish's push target and `remote`'s remote-template clone source share one redirectable definition. `git` is a subprocess and cannot be intercepted by an HTTP mock; redirecting this base at a directory of local bare repos is what makes those paths testable offline
7. Both bases accept a runtime override from the environment (`FLEDGE_TEST_GITHUB_API_BASE`, `FLEDGE_TEST_GITHUB_REMOTE_BASE`) because the `cfg(test)` thread-local cannot reach a *spawned* binary, which is what integration tests drive. It is a test hook, not configuration, and is doubly gated: compiled out entirely in release builds (every shipped binary), and — in a debug build — honoured only for a loopback `http://` host (userinfo forms such as `http://127.0.0.1@evil.example` rejected) or, for the remote base, an existing absolute local directory. So no environment, hostile or otherwise, can redirect an `Authorization: Bearer <token>` request to a host off the machine; a rejected value is ignored with a warning and the production constant stands

## Behavioral Examples

### github_api_get — authenticated request
```
github_api_get("/repos/CorvidLabs/fledge/issues", Some(token), &[("state", "open")])
  -> GET https://api.github.com/repos/CorvidLabs/fledge/issues?state=open
  -> Authorization: Bearer <token>
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
| Not a git repo | `ensure_git_repo` outside a git worktree | Bail with "Not a git repository" |

## Dependencies

- `ureq` — HTTP client
- `search::urlencod` — URL parameter encoding for query strings

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 4 | 2026-07-30 | Add the crate-internal `api_base()` indirection so `github_api_get` (and the `publish` helpers built on it) can be tested against a loopback mock server (#447). Release-build URLs are unchanged |
| 3 | 2026-06-07 | Remove `ensure_claude_cli` — the AI path no longer shells out to the `claude` CLI (1.5.0 moved to direct HTTP via `corvid-ai`). Only `github_api_get` and `ensure_git_repo` remain |
| 2 | 2026-04-25 | v0.15 tight-core: remove `detect_repo`, `parse_repo_url`, `format_relative_time`, they only existed for the deleted `checks`/`issues`/`prs` commands and now live in `fledge-plugin-github`. `parse_repo_url` retained as a `#[cfg(test)]` helper. |
| 1 | 2026-04-21 | Add ensure_git_repo and ensure_claude_cli exports |
| 1 | 2026-04-19 | Initial spec |
| 5 | 2026-08-08 | CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p: Mocking harness, HOME isolation and real publish coverage for network-touching paths |
