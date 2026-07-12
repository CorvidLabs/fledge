---
spec: github.spec.md
---

## User Stories

- As a module author, I want to call `detect_repo()` to get the GitHub owner/repo without parsing remotes myself
- As a module author, I want `github_api_get` to handle authentication and error formatting so I can focus on business logic
- As a developer, I want clear error messages when my token is missing or rate-limited

## Durable Requirements

### REQ-github-001

The implementation SHALL satisfy the following criterion: `detect_repo` parses HTTPS remote URLs (`https://github.com/owner/repo.git`)

Acceptance Criteria

- `detect_repo` parses HTTPS remote URLs (`https://github.com/owner/repo.git`)

### REQ-github-002

The implementation SHALL satisfy the following criterion: `detect_repo` parses SSH remote URLs (`git@github.com:owner/repo.git`)

Acceptance Criteria

- `detect_repo` parses SSH remote URLs (`git@github.com:owner/repo.git`)

### REQ-github-003

The implementation SHALL satisfy the following criterion: `detect_repo` handles token-authenticated HTTPS URLs

Acceptance Criteria

- `detect_repo` handles token-authenticated HTTPS URLs

### REQ-github-004

The implementation SHALL satisfy the following criterion: `detect_repo` strips trailing `.git` suffix

Acceptance Criteria

- `detect_repo` strips trailing `.git` suffix

### REQ-github-005

The implementation SHALL satisfy the following criterion: `github_api_get` reads token from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN`, or config

Acceptance Criteria

- `github_api_get` reads token from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN`, or config

### REQ-github-006

The implementation SHALL satisfy the following criterion: `github_api_get` returns parsed JSON on success

Acceptance Criteria

- `github_api_get` returns parsed JSON on success

### REQ-github-007

The implementation SHALL satisfy the following criterion: 403 responses produce a message about setting a token

Acceptance Criteria

- 403 responses produce a message about setting a token

### REQ-github-008

The implementation SHALL satisfy the following criterion: 404 responses produce a "Not found" error

Acceptance Criteria

- 404 responses produce a "Not found" error

### REQ-github-009

The implementation SHALL satisfy the following criterion: `format_relative_time` converts ISO 8601 timestamps to human-readable relative times

Acceptance Criteria

- `format_relative_time` converts ISO 8601 timestamps to human-readable relative times

### REQ-github-010

The implementation SHALL satisfy the following criterion: `format_relative_time` falls back to the raw string for unparseable input

Acceptance Criteria

- `format_relative_time` falls back to the raw string for unparseable input

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
