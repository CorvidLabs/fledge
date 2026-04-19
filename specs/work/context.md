---
spec: work.spec.md
---

## Context

Fledge is evolving from scaffolding to a project lifecycle tool. `fledge work` provides the git workflow layer — creating branches with consistent naming and streamlining PR creation. This reduces the friction of the feature branch workflow to two commands: `start` and `pr`.

## Related Modules

- `config` — could eventually store default base branch preference

## Design Decisions

- Shell out to `git` and `gh` rather than using libgit2 — keeps binary small and avoids linking issues
- Branch prefix is `feat/` by default — matches most team conventions
- Sanitize branch names (lowercase, replace non-alphanumeric with hyphens) to avoid git issues
