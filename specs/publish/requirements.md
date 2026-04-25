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
