---
spec: checks.spec.md
---

## Key Decisions

- Uses the GitHub Check Runs API (`/repos/{owner}/{repo}/commits/{branch}/check-runs`) rather than the older Statuses API — Check Runs provide richer data including duration and conclusion
- Defaults to the current branch via `git rev-parse` — no config needed for the common case
- Cancelled checks count as failed (not pending) because they represent a definitive non-success outcome
- Skipped checks count as passed because they indicate an intentionally bypassed (not failing) check
- `--json` outputs raw API response for scripting — no fledge-specific wrapping

## Files to Read First

- `src/checks.rs` — all logic lives here: API call, status mapping, formatting
- `src/github.rs` — `detect_repo()` and `github_api_get()` shared helpers
- `specs/checks/checks.spec.md` — formal API and invariants

## Current Status

- Fully implemented: branch detection, check fetching, status display with timing
- Supports all GitHub Check Run conclusions: success, failure, cancelled, skipped, and in-progress
- JSON output for scripting

## Notes

- Duration is calculated from `started_at` and `completed_at` timestamps — shows "running..." for in-progress checks
- The `format_duration` helper formats seconds as "12s" or "1m 30s"
