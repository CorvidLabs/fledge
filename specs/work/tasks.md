---
spec: work.spec.md
---

## Tasks

- [x] Write spec files for work module
- [x] Implement `work start` — branch creation with conventions
- [x] Implement `work pr` — push and PR creation via gh
- [x] Implement `work status` — branch and PR status
- [x] Wire up CLI subcommands in main.rs
- [x] Write unit tests
- [x] Run verification suite
- [x] Add flexible branch types (--branch-type flag)
- [x] Add issue linking (--issue flag)
- [x] Add custom prefix override (--prefix flag)
- [x] Add WorkConfig with fledge.toml [work] section support
- [x] Update spec to v2 for flexible branch types
- [x] Add `--json` to `start`, `pr`, `status` (v6): pretty output suppressed in JSON mode; errors still surface on stderr; exit codes unchanged
- [x] Add `behind` count to `status` JSON output (uses `git rev-list --count branch..base`)
- [x] `extract_pr_number` helper parses the trailing `/<n>` from `gh pr create`'s URL output

## Gaps

- No JSON option for `init` / `publish` / `create-template` / plugins create/publish — these are once-per-project commands and currently remain pretty-only
- `status --json` omits detached-HEAD and no-upstream edge cases in the payload schema (they still bail with an error, so agents can detect via non-zero exit)
- `pr --json` still runs the git push and gh pr create through the external processes; no capture of their intermediate stderr in the JSON (stderr still prints on failure)
