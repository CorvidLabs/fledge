---
module: work
version: 9
status: active
files:
  - src/work.rs

db_tables: []
depends_on:
  - config
  - llm
---

# Work

## Purpose

Provides opinionated git workflow commands for feature branch development. `fledge work start` creates a branch following configurable naming conventions (type, issue linking, custom prefix), and `fledge work pr` creates a pull request from the current branch with automatic title/body generation.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate work subcommand |
| `WorkAction` | Enum of subcommands: Start, Pr, Status |
| `sanitize_branch_name` | Normalizes a string into a valid git branch name (lowercase, hyphens, no leading/trailing hyphens) |
| `generate_title_from_branch` | Generates a human-readable PR title from a branch name by stripping any known type prefix and converting hyphens to spaces |
| `generate_body_from_commits` | Generates a Markdown PR body from `git log base..branch` — `## Summary` heading + one bullet per commit subject (conventional-commit prefix stripped) |
| `build_branch_name` | (test-only) Constructs a branch name from components using WorkConfig |

### Structs & Enums

| Type | Description |
|------|-------------|
| `WorkAction` | Enum of subcommands: Start (name, branch_type, issue, prefix, base, json), Pr (title, body, draft, base, json, yes, ai, provider, model), Status (json) |
| `WorkConfig` | Deserializable config with `branch_format` and `default_type` fields |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(WorkAction) -> Result<()>` | Dispatches to start, pr, or status |
| `start` | `(name, branch_type, issue, prefix, base, json) -> Result<()>` | Creates and checks out a branch with configurable type and naming |
| `pr` | `(title, body, draft, base, json, yes, ai, provider, model) -> Result<()>` | Creates a PR via `gh` CLI; auto-generates title/body and shows a preview + confirmation unless `--yes` or `--json`. `--ai` switches the body generator to the configured LLM provider |
| `status` | `(json: bool) -> Result<()>` | Shows current branch, commits ahead/behind, and PR status |
| `sanitize_branch_name` | `(&str) -> String` | Lowercase, replace special chars with hyphens, collapse consecutive hyphens |
| `generate_title_from_branch` | `(&str) -> String` | Strip type prefix, convert hyphens to spaces, title-case |
| `generate_body_from_commits` | `(branch, base) -> Result<String>` | Build `## Summary` body from `base..branch` commit subjects; strips conventional-commit prefixes |
| `build_branch_name` | `(name, branch_type, issue, prefix, config) -> String` | Apply format template with `{author}`, `{type}`, `{name}`, `{issue}` substitution |

**Internal (not exported):**
- `load_work_config() -> WorkConfig` — Reads `[work]` section from `fledge.toml`, falls back to defaults
- `format_commit_subject_as_bullet(&str) -> String` — Strips a leading conventional-commit prefix and upper-cases the first letter
- `print_pr_preview(title, body, head, base, draft)` — Renders the boxed preview block
- `generate_body_with_ai(branch, base, provider, model, json) -> Result<String>` — Builds a context bundle (commit log + diffstat + truncated diff) and asks the configured LLM for a Markdown PR body

## Invariants

1. Branch names are normalized via `sanitize_branch_name`: lowercase, special chars become hyphens, no consecutive or trailing hyphens
2. Default branch format is `{author}/{type}/{name}` — configurable via `[work]` in `fledge.toml`
3. Valid branch types: `feat`, `feature`, `fix`, `bug`, `chore`, `task`, `docs`, `hotfix`, `refactor` (enforced unless `--prefix` is used)
4. Default branch type is `feat` — configurable via `[work] default_type` in `fledge.toml`
5. `{author}` resolves from global config `defaults.author` or `git config user.name`
6. `start` refuses to create a branch if there are uncommitted changes
7. `pr` requires `gh` CLI to be installed and authenticated
8. `pr` pushes the current branch to origin before creating the PR
9. `status` works without `gh` (gracefully degrades if not available)
10. `--prefix` bypasses type validation and format template, using raw `prefix/name`
11. `--issue N` prepends the issue number to the branch name segment: `N-name`
12. `generate_title_from_branch` strips any valid branch type prefix (feat/, feature/, fix/, bug/, chore/, task/, docs/, hotfix/, refactor/)
13. Plugin lifecycle hook `post_work_start` runs after branch creation (errors silently ignored via `.ok()`)
14. Plugin lifecycle hook `pre_pr` runs before PR creation (errors propagate and abort the PR)
15. `--json` on `start` emits `{branch, base, type, prefix, issue}` and suppresses the pretty ✅ output
16. `--json` on `pr` emits `{url, number, title, head, base, draft}` and suppresses spinner + pretty output
17. `--json` on `status` emits `{branch, default, ahead, behind, pr: {number, state, url} | null}`. `behind` is `null` when `git rev-list` can't compute it (e.g. the base hasn't been fetched) — this is distinct from `0` (up-to-date) so agents can detect "needs fetch" vs "up to date"
18. `--json` never silences errors — error messages still go to stderr; exit code is still non-zero on failure
19. `pr` auto-generates the body from `git log base..branch` when `--body` is omitted; conventional-commit prefixes (`feat:`, `fix(scope):`, etc.) are stripped and bullets read as sentences. If no commits between base and branch, falls back to a `(describe the change)` placeholder
20. `pr` shows a styled preview of the title, branch flow, and full body before invoking `gh pr create`; the preview is suppressed in `--json` mode
21. `pr` prompts `Create this pull request?` with default Yes after the preview. `--yes` / `-y` skips the prompt; `--json` skips it as well (assumes agent intent). In a non-interactive shell without `--yes` or `--json`, `pr` bails rather than hanging
22. Choosing "No" at the confirmation prompt aborts cleanly with a `✋ Aborted.` message and exits 0 without pushing or calling `gh`
23. `--ai` replaces heuristic body generation with an LLM call via `crate::llm::build_provider`; the prompt includes the full commit log, `git diff --stat`, and the diff itself (truncated to 600 lines so small/local models stay in context). `--provider` / `--model` override the active selection for this single call. `--body <text>` always wins over `--ai` (literal beats generated)
24. The AI body generation shows a "Drafting PR body [provider (model)]:" spinner unless `--json` is set; the resulting body still flows through the same preview + confirmation gate, so the user always sees what will be posted before it goes

## Behavioral Examples

### work start — create branch with default type
```
$ fledge work start add-search
✓ Created branch leif/feat/add-search from main
✓ Switched to leif/feat/add-search
```

### work start — with explicit type
```
$ fledge work start login-crash --branch-type fix
✓ Created branch leif/fix/login-crash from main
✓ Switched to leif/fix/login-crash
```

### work start — with issue number
```
$ fledge work start login-crash --branch-type fix --issue 42
✓ Created branch leif/fix/42-login-crash from main
✓ Switched to leif/fix/42-login-crash
```

### work start — with custom prefix (bypasses format)
```
$ fledge work start experiment --prefix sandbox
✓ Created branch sandbox/experiment from main
✓ Switched to sandbox/experiment
```

### work start — with custom base
```
$ fledge work start fix-bug --base develop
✓ Created branch leif/feat/fix-bug from develop
✓ Switched to leif/feat/fix-bug
```

### work start — dirty working tree
```
$ fledge work start new-feature
error: uncommitted changes detected. Commit or stash before starting work.
```

### work start — invalid branch type
```
$ fledge work start foo --type yolo
error: Unknown branch type 'yolo'. Valid types: feat, feature, fix, bug, chore, task, docs, hotfix, refactor
```

### work pr — create pull request (with preview + confirmation)
```
$ fledge work pr

────────────────────────────────────────────────────────────
Title: Add search command
Branch:  leif/feat/add-search → main

  ## Summary

  - Add search command
  - Wire up search index

────────────────────────────────────────────────────────────
? Create this pull request? (Y/n) y
✓ Pushed leif/feat/add-search to origin
✓ Created PR #42: "Add search command"
  https://github.com/owner/repo/pull/42
```

### work pr — skip the prompt
```
$ fledge work pr --yes
… preview …
✓ Pushed leif/feat/add-search to origin
✓ Created PR #42: "Add search command"
  https://github.com/owner/repo/pull/42
```

### work pr — with title and draft
```
$ fledge work pr --title "WIP: search command" --draft --yes
✓ Pushed leif/feat/add-search to origin
✓ Created draft PR #42: "WIP: search command"
  https://github.com/owner/repo/pull/42
```

### work pr — AI-generated body
```
$ fledge work pr --ai
✓ Drafting PR body [ollama (qwen3-coder:480b-cloud)]: 6.2s

────────────────────────────────────────────────────────────
Title: Add search command
Branch:  leif/feat/add-search → main

  ## Summary
  - Adds a `fledge search` subcommand backed by a new `SearchIndex` …
  - Wires the index into the existing `init` flow so templates …

  ## Test plan
  - [ ] `fledge search "hello"` returns the expected hits
  - [ ] Empty index does not panic on lookup

────────────────────────────────────────────────────────────
? Create this pull request? (Y/n) y
✓ Pushed leif/feat/add-search to origin
✓ Created PR #42: "Add search command"
```

### work pr — pin a specific provider/model just for this PR
```
$ fledge work pr --ai --provider ollama --model gpt-oss:120b-cloud --yes
… preview …
✓ Pushed leif/feat/add-search to origin
✓ Created PR #43: "Add search command"
```

### work status — on feature branch
```
$ fledge work status
  Branch: leif/feat/add-search (3 commits ahead of main)
  PR: #42 (open) — https://github.com/owner/repo/pull/42
```

### work start --json
```
$ fledge work start add-search --json
{
  "branch": "leif/feat/add-search",
  "base": "main",
  "type": "feat",
  "prefix": null,
  "issue": null
}
```

### work pr --json
```
$ fledge work pr --json
{
  "url": "https://github.com/owner/repo/pull/42",
  "number": 42,
  "title": "Add search command",
  "head": "leif/feat/add-search",
  "base": "main",
  "draft": false
}
```

### work status --json
```
$ fledge work status --json
{
  "branch": "leif/feat/add-search",
  "default": "main",
  "ahead": 3,
  "behind": 0,
  "pr": {
    "number": 42,
    "state": "open",
    "url": "https://github.com/owner/repo/pull/42"
  }
}
```

### work status --json — when base hasn't been fetched
```
$ fledge work status --json
{
  "branch": "leif/feat/add-search",
  "default": "main",
  "ahead": 3,
  "behind": null,
  "pr": null
}
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Not a git repository | Any subcommand outside git repo | Bail with message |
| Uncommitted changes | `work start` with dirty tree | Bail with message |
| Branch already exists | `work start` with existing branch name | Bail with message |
| Unknown branch type | `work start --branch-type <invalid>` without `--prefix` | Bail with valid types list |
| On main/master | `work pr` from default branch | Bail with message |
| `gh` not installed | `work pr` | Bail with install instructions |
| No commits ahead | `work pr` with no new commits | Bail with message |
| Non-interactive without --yes | `work pr` in a non-TTY shell without `--yes` or `--json` | Bail asking for `--yes` rather than hanging on the prompt |
| User declines confirmation | `work pr` and answers "n" at the prompt | Print `✋ Aborted.` and exit 0 without pushing |
| AI body generation fails | `--ai` with provider unreachable or model timeout | Bubble up the provider error verbatim (same surface as `fledge ask`) so the user can fix the config and retry |

## Dependencies

- `console` — styled terminal output
- `dialoguer` — `Confirm` prompt for the PR preview confirmation
- `serde` / `toml` — config deserialization for `[work]` section in `fledge.toml`
- `crate::config::Config` — global config for author resolution
- `crate::utils::is_interactive` — TTY gate for the confirmation prompt
- `crate::llm::{build_provider, ProviderOverride, describe}` — `--ai` body generation
- `crate::spinner::Spinner` — progress indicator during the AI call
- Git CLI — branch operations
- `gh` CLI — PR creation (optional for `status`)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec for fledge work |
| 2 | 2026-04-20 | Flexible branch types, configurable format, `{author}` support, `--type`/`--issue`/`--prefix` flags |
| 3 | 2026-04-20 | Added `feature`, `bug`, `task` as valid branch types |
| 4 | 2026-04-21 | Correct load_work_config as internal (not exported) |
| 5 | 2026-04-22 | Document lifecycle hooks: post_work_start (silent) and pre_pr (propagating) |
| 6 | 2026-04-23 | Add `--json` to `start`, `pr`, and `status`. `status` now also reports `behind`. Pretty output suppressed in JSON mode; errors still go to stderr. |
| 7 | 2026-04-24 | `work pr` auto-generates the PR body from commits when `--body` is omitted, shows a styled preview, and prompts for confirmation before creating the PR. `--yes` / `-y` skips the prompt; non-interactive shells must pass `--yes` or `--json`. |
| 8 | 2026-04-24 | `work pr --ai` generates a richer Markdown body via the configured LLM (`fledge ai use`-aware), with `--provider` / `--model` per-call overrides. Prompt includes commit log, diffstat, and truncated diff; spinner shown unless `--json`. |
| 9 | 2026-04-26 | Tier-D 1.0 envelope: `work start --json`, `work pr --json`, `work status --json` now include `schema_version: 1` and `action: "work_start"|"work_pr"|"work_status"` at the top level. Field shapes otherwise unchanged. Closes the gap where tier C (#274) only migrated plugins/lanes/templates and missed the cross-cutting commands |
