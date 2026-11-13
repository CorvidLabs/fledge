---
id: run-cargo-audit-directly-in-the-ci-audit-job
state: draft
type: operations
base_commit: add5767b1b0b4b3e0bdf4db0d9f6975de4daef4e
---

# Run cargo-audit directly in the CI audit job

## Intent

Run cargo-audit directly in the CI audit job

## Affected Canonical Specs

- None

## Acceptance Criteria

- The CI audit job installs cargo-audit and runs cargo audit after cargo generate-lockfile, with no rustsec/audit-check step; the job still fails on any vulnerability advisory; unmaintained, unsound or yanked warnings are printed without failing the job; the job id and name stay audit so the required check and the corvid-pet and attest needs are unchanged

## No-spec Rationale

rustsec/audit-check v2.0.0 turns any unmaintained or unsound warning into a Checks API report whose update returns 404 (Not Found), so the required audit check fails with 0 vulnerabilities (seen on CorvidLabs/fledge#547). Running cargo audit directly keeps the same gate: it fails on vulnerabilities and only prints warnings.
