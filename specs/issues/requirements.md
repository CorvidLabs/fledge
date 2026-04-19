---
spec: issues.spec.md
---

## User Stories

- As a developer, I want to run `fledge issues` to see open issues for my repo without leaving the terminal
- As a developer, I want to run `fledge issues view 34` to read a specific issue's details
- As a developer, I want to filter issues by label or state
- As a developer, I want JSON output for scripting

## Acceptance Criteria

- `fledge issues` lists open issues for the current repo, sorted by recently updated
- Pull requests are filtered out of the listing (GitHub API returns both)
- `fledge issues --state closed` shows closed issues
- `fledge issues --label bug` filters by label
- `fledge issues --limit N` caps the result count
- `fledge issues --json` outputs raw JSON
- `fledge issues view <number>` displays issue title, state, author, labels, comments, and body
- `fledge issues view` with a PR number suggests `fledge prs` instead

## Constraints

- Requires a GitHub remote origin
- Uses `github` module for API calls — no direct HTTP

## Out of Scope

- Creating or editing issues
- Assigning or labeling issues
- Issue search across repos
