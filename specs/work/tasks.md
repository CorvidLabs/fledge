---
spec: work.spec.md
---

## Tasks

- [x] Write spec files for work module
- [x] Implement `work start` — branch creation with conventions
- [x] Implement `work status` — branch and PR status
- [x] Wire up CLI subcommands in main.rs
- [x] Write unit tests
- [x] Run verification suite
- [x] Add flexible branch types (--branch-type flag)
- [x] Add issue linking (--issue flag)
- [x] Add custom prefix override (--prefix flag)
- [x] Add WorkConfig with fledge.toml [work] section support
- [x] Update spec to v2 for flexible branch types
- [x] Add `--json` to `start`, `pr`, `status` (v6)
- [x] Add `behind` count to `status` JSON output
- [x] `extract_pr_number` helper parses PR URL
- [x] **v7: Remove `work pr`** — PR creation moved to `fledge-plugin-github`
- [x] **v7: Add `work commit`** — stage + conventional commit with `--ai` support
- [x] **v7: Add `work push`** — push branch to origin with tracking
- [x] **v7: Rework `work status`** — pure git, no `gh` dependency

## Gaps

- `commit --ai` prompt could be improved with project-specific context (e.g. reading CONTRIBUTING.md for commit conventions)
- No `--amend` flag on commit (intentionally excluded to keep scope tight)
