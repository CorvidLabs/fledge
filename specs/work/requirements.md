---
spec: work.spec.md
---

## User Stories

- As a developer, I want to run `fledge work start my-feature` to create a properly named feature branch
- As a developer, I want to specify `--branch-type fix` to create a fix branch instead of the default feat
- As a developer, I want to link an issue with `--issue 42` so my branch includes the issue number
- As a developer, I want `--prefix user/leif` to override the branch format entirely
- As a developer, I want to configure the default branch format in `fledge.toml`
- As a developer, I want `fledge work pr` to push and create a PR in one command
- As a developer, I want `fledge work status` to see my branch state and PR link
- As a developer, I want `--draft` to create draft PRs for work in progress

## Acceptance Criteria

- `fledge work start <name>` creates a branch using the configured format (default: `{author}/{type}/{name}`)
- `fledge work start <name> --branch-type fix` creates a fix-type branch
- `fledge work start <name> --issue 42` includes issue number in branch name
- `fledge work start <name> --prefix user/leif` creates `user/leif/<name>` branch
- `fledge work start` refuses if working tree is dirty
- `fledge work start` rejects invalid branch types (not in feat, fix, chore, docs, hotfix, refactor)
- `fledge work pr` pushes the branch and creates a PR via `gh`
- `fledge work pr --title "X"` uses the given title
- `fledge work pr --draft` creates a draft PR
- `fledge work pr --base develop` targets a non-default base branch
- `fledge work status` shows branch name, commits ahead, and PR info
- Branch names are sanitized (lowercase, hyphens only)
- `[work]` section in `fledge.toml` can override `branch_format` and `default_type`

## Constraints

- Requires git CLI for all operations
- Requires `gh` CLI for PR creation
- Must work on macOS and Linux

## Out of Scope

- Interactive rebase or squash
- Branch cleanup or deletion
