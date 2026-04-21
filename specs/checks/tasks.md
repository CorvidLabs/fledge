---
spec: checks.spec.md
---

## Tasks

- [x] Write checks spec
- [x] Implement ChecksOptions struct with branch and json fields
- [x] Implement run() entry point with GitHub Check Runs API call
- [x] Implement branch detection (current branch or --branch override)
- [x] Implement check run display with status icons, names, and durations
- [x] Implement summary line with pass/fail/pending counts
- [x] Implement --json output for raw API response
- [x] Handle detached HEAD with helpful error message
- [x] Wire ChecksAction subcommand into main.rs
- [x] Add unit tests
- [x] Register spec and verify with cargo test, clippy, fmt, spec-check

## Gaps

- No filtering by check name or status
- No watch/polling mode to wait for checks to complete
