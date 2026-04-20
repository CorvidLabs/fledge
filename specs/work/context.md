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
