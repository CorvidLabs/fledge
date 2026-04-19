---
spec: github.spec.md
---

## User Stories

- As a module author, I want to call `detect_repo()` to get the GitHub owner/repo without parsing remotes myself
- As a module author, I want `github_api_get` to handle authentication and error formatting so I can focus on business logic
- As a developer, I want clear error messages when my token is missing or rate-limited

## Acceptance Criteria

- `detect_repo` parses HTTPS remote URLs (`https://github.com/owner/repo.git`)
- `detect_repo` parses SSH remote URLs (`git@github.com:owner/repo.git`)
- `detect_repo` handles token-authenticated HTTPS URLs
- `detect_repo` strips trailing `.git` suffix
- `github_api_get` reads token from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN`, or config
- `github_api_get` returns parsed JSON on success
- 403 responses produce a message about setting a token
- 404 responses produce a "Not found" error
- `format_relative_time` converts ISO 8601 timestamps to human-readable relative times
- `format_relative_time` falls back to the raw string for unparseable input

## Constraints

- Uses `ureq` for HTTP — no async runtime required
- No direct dependency on `gh` CLI

## Out of Scope

- GraphQL API support
- Pagination of API results
- Write operations (POST/PATCH/DELETE)
