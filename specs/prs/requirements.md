---
spec: prs.spec.md
---

## User Stories

- As a developer, I want to run `fledge prs` to see open pull requests for my repo
- As a developer, I want to run `fledge prs view 45` to see PR details including diff stats
- As a developer, I want to distinguish draft PRs from ready PRs at a glance
- As a developer, I want JSON output for scripting

## Acceptance Criteria

- `fledge prs` lists open PRs sorted by recently updated
- Draft PRs show a distinct icon (open circle) vs ready PRs (filled circle)
- Merged PRs display "merged" state in magenta
- `fledge prs --state closed` shows closed/merged PRs
- `fledge prs --limit N` caps the result count
- `fledge prs --json` outputs raw JSON
- `fledge prs view <number>` shows title, state, author, branch info, diff stats, comments, and body

## Constraints

- Requires a GitHub remote origin
- Uses `github` module for API calls — no direct HTTP

## Out of Scope

- Creating or merging PRs (handled by `fledge work pr`)
- Reviewing or commenting on PRs
- CI status checks
