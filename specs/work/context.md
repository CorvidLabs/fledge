---
spec: work.spec.md
---

## Context

Fledge is evolving from scaffolding to a project lifecycle tool. `fledge work` provides the git workflow layer — creating branches with consistent naming and streamlining PR creation. This reduces the friction of the feature branch workflow to two commands: `start` and `pr`.

## Related Modules

- `config` — provides `author_or_git()` used in branch format string interpolation

## Design Decisions

- Shell out to `git` and `gh` rather than using libgit2 — keeps binary small and avoids linking issues
- Branch type defaults to `feat` but supports feat, fix, chore, docs, hotfix, refactor
- Branch format is configurable via `[work]` section in `fledge.toml` using template variables: `{author}`, `{type}`, `{issue}`, `{name}`
- `--prefix` flag bypasses the format string entirely for teams with non-standard conventions
- `--issue` flag prepends the issue number to the name portion for GitHub issue linking
- Sanitize branch names (lowercase, replace non-alphanumeric with hyphens) to avoid git issues
- `generate_title_from_branch` dynamically strips any known type prefix rather than hardcoding a few
- **v6: `--json` added to all three subcommands** so agents can drive the workflow without parsing human-formatted output. Rationale: agents using `fledge work start` need the branch name back; agents using `fledge work pr` need the PR URL and number; `fledge work status` should give a shape compatible with the GitHub-plugin commands `fledge checks --json` and `fledge prs --json` (provided by `fledge-plugin-github`). Each JSON payload is a single top-level object.
- In JSON mode, spinners and ✅ output are suppressed entirely so the only stdout content is the JSON payload. Errors still go to stderr, and the process still exits non-zero on failure — so `if fledge work pr --json; then ... fi` works as expected.
- `status` now also reports `behind` (commits the base has that the branch doesn't). Cheap and agent-useful.
