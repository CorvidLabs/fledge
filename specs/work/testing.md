---
spec: work.spec.md
---

## Test Plan

### Unit Tests

- Branch name sanitization (spaces, special chars, uppercase)
- Default branch detection
- Commit count parsing
- Title generation strips all valid type prefixes (feat, fix, chore, docs, hotfix, refactor)
- `build_branch_name` with default format
- `build_branch_name` with fix type
- `build_branch_name` with issue number
- `build_branch_name` with custom prefix
- `build_branch_name` with custom format string
- `WorkConfig` default values
- `WorkConfig` parsing from TOML (full, partial, missing section)
- Valid branch types list
- `build_commit_message` formats type + message correctly
- `build_commit_message` capitalizes first letter of message
- `build_commit_message` uses default type when none specified

### Integration Tests

- `fledge work start` creates a branch in a temp git repo
- `fledge work commit` creates a commit with conventional format
- `fledge work push` pushes branch to origin
- `fledge work status` shows branch info without gh dependency
