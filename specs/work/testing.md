---
spec: work.spec.md
---

## Test Plan

### Unit Tests

- Branch name sanitization (spaces, special chars, uppercase)
- Default branch detection
- Commit count parsing
- PR URL extraction from gh output
- Title generation strips all valid type prefixes (feat, fix, chore, docs, hotfix, refactor)
- `build_branch_name` with default format
- `build_branch_name` with fix type
- `build_branch_name` with issue number
- `build_branch_name` with custom prefix
- `build_branch_name` with custom format string
- `WorkConfig` default values
- `WorkConfig` parsing from TOML (full, partial, missing section)
- Valid branch types list

### Integration Tests

- `fledge work start` creates a branch in a temp git repo
- `fledge work status` shows branch info
