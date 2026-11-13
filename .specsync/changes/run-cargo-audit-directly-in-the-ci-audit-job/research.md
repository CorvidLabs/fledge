---
change: run-cargo-audit-directly-in-the-ci-audit-job
artifact: research
---

# Research

- rustsec/audit-check v2.0.0 `src/main.ts`: any vulnerability or warning sets `shouldReport`, and
  every event except `schedule` calls `reportCheck`.
- `src/reporter.ts` `reportCheck`:
  - `startCheck('queued')` creates the check run. If that fails on a pull request, the action
    falls back to printing the report.
  - `finishCheck` updates the run. If that fails, the error is rethrown and fails the job.
  - It fails the job on its own only when there is a non-informational vulnerability
    (`stats.critical > 0`). Unmaintained, unsound and notice advisories never fail it.
- `cargo audit` (cargo-audit 0.22 and 0.23) exits non-zero on vulnerabilities. It reports
  unmaintained, unsound and yanked warnings without failing unless `--deny` is passed. That is
  the same pass/fail rule, with no API call.
- A local run on 2026-11-13 (`cargo generate-lockfile && cargo audit`, cargo-audit 0.22.2,
  367 crates) exited 0 with "2 allowed warnings found" (the same two advisories).
