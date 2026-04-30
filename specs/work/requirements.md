---
spec: work.spec.md
---

## User Stories

- As a developer, I want to run `fledge work start my-feature` to create a properly named feature branch
- As a developer, I want to specify `--branch-type fix` to create a fix branch instead of the default feat
- As a developer, I want to link an issue with `--issue 42` so my branch includes the issue number
- As a developer, I want `--prefix user/leif` to override the branch format entirely
- As a developer, I want to configure the default branch format in `fledge.toml`
- As a developer, I want `fledge work commit` to stage and commit with a conventional-commit message
- As a developer, I want `fledge work commit --ai` to auto-generate a commit message from the staged diff
- As a developer, I want `fledge work push` to push my branch to origin with tracking
- As a developer, I want `fledge work status` to see my branch state (ahead/behind, dirty files)

## Acceptance Criteria

- `fledge work start <name>` creates a branch using the configured format (default: `{author}/{type}/{name}`)
- `fledge work start <name> --branch-type fix` creates a fix-type branch
- `fledge work start <name> --issue 42` includes issue number in branch name
- `fledge work start <name> --prefix user/leif` creates `user/leif/<name>` branch
- `fledge work start` refuses if working tree is dirty
- `fledge work start` rejects invalid branch types (not in feat, feature, fix, bug, chore, task, docs, hotfix, refactor)
- `fledge work commit` stages all changes and creates a commit (prompts for type + message interactively)
- `fledge work commit -m "message"` uses the given message with default type
- `fledge work commit --type fix -m "null pointer"` creates `fix: null pointer` commit
- `fledge work commit --ai` generates the commit message from staged diff using the configured AI provider
- `fledge work commit` refuses if there are no changes to commit
- `fledge work push` pushes the current branch to origin with `-u` tracking
- `fledge work push` refuses if on the default branch
- `fledge work push` refuses if there are no commits ahead of the remote
- `fledge work status` shows branch name, commits ahead/behind, and uncommitted file count
- `fledge work status` does NOT call `gh` or any GitHub API — pure git only
- Branch names are sanitized (lowercase, hyphens only)
- `[work]` section in `fledge.toml` can override `branch_format` and `default_type`
- All subcommands support `--json` for agent consumption

## Constraints

- Requires git CLI for all operations
- No GitHub CLI or API dependency — PR creation lives in `fledge-plugin-github`
- Must work on macOS and Linux

## Out of Scope

- Interactive rebase or squash
- Branch cleanup or deletion
- PR creation (moved to `fledge-plugin-github`)
