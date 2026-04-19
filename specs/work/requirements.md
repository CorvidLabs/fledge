---
spec: work.spec.md
---

## User Stories

- As a developer, I want to run `fledge work start my-feature` to create a properly named feature branch
- As a developer, I want `fledge work pr` to push and create a PR in one command
- As a developer, I want `fledge work status` to see my branch state and PR link
- As a developer, I want `--draft` to create draft PRs for work in progress

## Acceptance Criteria

- `fledge work start <name>` creates `feat/<name>` branch from main (or `--base`)
- `fledge work start` refuses if working tree is dirty
- `fledge work pr` pushes the branch and creates a PR via `gh`
- `fledge work pr --title "X"` uses the given title
- `fledge work pr --draft` creates a draft PR
- `fledge work pr --base develop` targets a non-default base branch
- `fledge work status` shows branch name, commits ahead, and PR info
- Branch names are sanitized (lowercase, hyphens only)

## Constraints

- Requires git CLI for all operations
- Requires `gh` CLI for PR creation
- Must work on macOS and Linux

## Out of Scope

- Interactive rebase or squash
- Branch cleanup or deletion
- Issue linking automation
