---
spec: checks.spec.md
---

## User Stories

- As a developer, I want to see CI check status for my current branch without leaving the terminal
- As a developer, I want to check CI status for a specific branch with `--branch`
- As a developer, I want machine-readable output with `--json` for scripting

## Acceptance Criteria

- `fledge checks` shows check name, status icon, and duration for each check run
- `fledge checks --branch <name>` queries checks for the specified branch
- Without `--branch`, defaults to the current git branch
- Shows a summary line with pass/fail/pending counts
- `--json` outputs the raw GitHub API response
- Detached HEAD without `--branch` produces a clear error message

## Constraints

- Requires a GitHub remote (`origin`) to detect owner/repo
- GitHub token from config is used if available (unauthenticated requests are rate-limited)

## Out of Scope

- Watching/polling for check completion
- Triggering re-runs of failed checks
- GitHub Actions workflow-level status (only check runs)
