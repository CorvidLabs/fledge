---
kind: session
id: 2026-11-14-549
aliases: [549]
objective: "Review and close out open PR #549 (corvid/fix-audit-check-reporting): CI run cargo-audit directly so a Checks API 404 can't fail the audit gate"
intent_scope: ""
modules: []
stage: intent
stage_status: active
iteration: 1
callers: []
native_change: ""
pr: ""
criteria: []
requirements: []
cases: []
design: []
files: []
observations: []
bugs: []
blockers: []
waiting_on_human: []
run_consents: []
next: Hold for the human implementation review at the accept stage; then chough change ship run-cargo-audit-directly-in-the-ci-audit-job --close-gate 'Implementation review of run-cargo-audit-directly-in-the-ci-audit-job' --decided-by 0xLeif --words <their words> --where <where> --recorded-by claude-code-t382
---
# Review and close out open PR #549 (corvid/fix-audit-check-reporting): CI run cargo-audit directly so a Checks API 404 can't fail the audit gate

## Goal

Review and close out open PR #549 (corvid/fix-audit-check-reporting): CI run cargo-audit directly so a Checks API 404 can't fail the audit gate

## Where it stands

- Reviewed open PR #549 (corvid/fix-audit-check-reporting): CI now runs cargo-audit directly instead of rustsec/audit-check@v2.0.0. CI audit job passed on the PR head (run 43452531050); local fledge lanes run verify green on the branch head (6 steps, success true).

## Decisions and why

- rustsec/audit-check v2.0.0 fails the required audit check on pull requests because its Checks API update returns 404; a direct cargo audit keeps the same fail-on-vulnerability gate without the API call, so the PR is the right fix.

## What was verified

## Open threads

## Timeline

- 2026-11-14T18:21:40Z · start · intent → intent · Review and close out open PR #549 (corvid/fix-audit-check-reporting): CI run cargo-audit directly so a Checks API 404 can't fail the audit gate ^t-021ef2
  - revision: add5767b1b0b4b3e0bdf4db0d9f6975de4daef4e
  - by: unknown agent (hc t1)
