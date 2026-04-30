---
spec: work.spec.md
---

## Context

Fledge is evolving from scaffolding to a project lifecycle tool. `fledge work` provides the git workflow layer — creating branches with consistent naming, committing with conventional messages, and pushing to the remote. PR creation lives in `fledge-plugin-github`, keeping `fledge work` free of any GitHub CLI or API dependency.

## Related Modules

- `config` — provides `author_or_git()` used in branch format string interpolation
- `llm` — AI provider for commit message generation (`--ai` flag on `commit`)
- `fledge-plugin-github` — PR creation, PR status, issues, checks (external plugin)

## Design Decisions

- Shell out to `git` rather than using libgit2 — keeps binary small and avoids linking issues
- Branch type defaults to `feat` but supports feat, fix, chore, docs, hotfix, refactor
- Branch format is configurable via `[work]` section in `fledge.toml` using template variables: `{author}`, `{type}`, `{issue}`, `{name}`
- `--prefix` flag bypasses the format string entirely for teams with non-standard conventions
- `--issue` flag prepends the issue number to the name portion for GitHub issue linking
- Sanitize branch names (lowercase, replace non-alphanumeric with hyphens) to avoid git issues
- `generate_title_from_branch` dynamically strips any known type prefix rather than hardcoding a few
- **v6: `--json` added to all subcommands** so agents can drive the workflow without parsing human-formatted output. Each JSON payload is a single top-level object with a `schema_version` field.
- In JSON mode, spinners and emoji output are suppressed entirely so the only stdout content is the JSON payload. Errors still go to stderr, and the process still exits non-zero on failure.
- **v7: Pure git split** — `fledge work` owns only git operations (start, commit, push, status). PR creation moved to `fledge-plugin-github` (not yet shipped; use `gh pr create` in the meantime). The `gh` CLI is no longer required by core fledge. `fledge work pr` was removed (prints deprecation notice directing to `gh pr create`).
- `commit` uses conventional-commit format (`type: message`). The `--ai` flag generates the message from the staged diff using the configured LLM provider.
- `push` uses `git push -u origin <branch>` to set up tracking. Refuses to push the default branch to prevent accidental main pushes.
- `status` reports branch name, ahead/behind counts, and uncommitted changes count — no GitHub API calls.
