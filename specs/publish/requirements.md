# Publish — Requirements

This module is a library — its requirements describe the helpers it exposes. The user-facing publish commands (`templates publish`, `lanes publish`, `plugins publish`) define their own validation and prompting in their respective specs.

## Functional Requirements

1. Resolve the authenticated GitHub username from a token (`get_authenticated_user`)
2. Check whether a `<owner>/<repo>` exists on GitHub (`check_repo_exists`) — false on 404, error on other non-2xx
3. Create a new GitHub repository under the user or an organization with optional private flag and description (`create_github_repo`)
4. Add a single topic to a repository's existing topic set, additively (`set_repo_topic`)
5. Initialize git (if needed), commit working-tree contents, and force-push to `origin/main` using one-shot HTTP-Basic auth (`push_directory`) — token is never persisted in `.git/config`
6. Surface clear error messages for the common API failures: 422 (name conflict), 403 (insufficient scope)

## Non-Functional Requirements

1. Synchronous HTTP via `ureq` — no async runtime
2. Token never reaches the process table or git config — passed via `http.extraheader` env injection only
3. No prompts in this module — caller decides whether to confirm

## Durable Requirements

### REQ-publish-001

The implementation SHALL satisfy the following criterion: Resolve the authenticated GitHub username from a token (`get_authenticated_user`)

Acceptance Criteria

- Resolve the authenticated GitHub username from a token (`get_authenticated_user`)

### REQ-publish-002

The implementation SHALL satisfy the following criterion: Check whether a `<owner>/<repo>` exists on GitHub (`check_repo_exists`) — false on 404, error on other non-2xx

Acceptance Criteria

- Check whether a `<owner>/<repo>` exists on GitHub (`check_repo_exists`) — false on 404, error on other non-2xx

### REQ-publish-003

The implementation SHALL satisfy the following criterion: Create a new GitHub repository under the user or an organization with optional private flag and description (`create_github_repo`)

Acceptance Criteria

- Create a new GitHub repository under the user or an organization with optional private flag and description (`create_github_repo`)

### REQ-publish-004

The implementation SHALL satisfy the following criterion: Add a single topic to a repository's existing topic set, additively (`set_repo_topic`)

Acceptance Criteria

- Add a single topic to a repository's existing topic set, additively (`set_repo_topic`)

### REQ-publish-005

The implementation SHALL satisfy the following criterion: Initialize git (if needed), commit working-tree contents, and force-push to `origin/main` using one-shot HTTP-Basic auth (`push_directory`) — token is never persisted in `.git/config`

Acceptance Criteria

- Initialize git (if needed), commit working-tree contents, and force-push to `origin/main` using one-shot HTTP-Basic auth (`push_directory`) — token is never persisted in `.git/config`

### REQ-publish-006

The implementation SHALL satisfy the following criterion: Surface clear error messages for the common API failures: 422 (name conflict), 403 (insufficient scope)

Acceptance Criteria

- Surface clear error messages for the common API failures: 422 (name conflict), 403 (insufficient scope)

### REQ-publish-007

The implementation SHALL satisfy the following criterion: Synchronous HTTP via `ureq` — no async runtime

Acceptance Criteria

- Synchronous HTTP via `ureq` — no async runtime

### REQ-publish-008

The implementation SHALL satisfy the following criterion: Token never reaches the process table or git config — passed via `http.extraheader` env injection only

Acceptance Criteria

- Token never reaches the process table or git config — passed via `http.extraheader` env injection only

### REQ-publish-009

The implementation SHALL satisfy the following criterion: No prompts in this module — caller decides whether to confirm

Acceptance Criteria

- No prompts in this module — caller decides whether to confirm
