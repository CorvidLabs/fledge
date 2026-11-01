---
change: run-the-specsync-archive-gate-before-every-ci-job
artifact: plan
---

# Plan

## Approach

Add one `archive-gate` job, displayed as `Archive`, at the top of `jobs:` in
every workflow that runs on `pull_request` or `push` to a branch, and make every
other job in those workflows depend on it. The gate checks out the pull request
head (or the pushed commit) without persisting credentials and runs
`CorvidLabs/spec-sync@106aaa43d0bcf049f19b173ddb63815a25c3f931` with
`archive-gate-only: 'true'`.

## Out of scope

- `release.yml` and `post-release-formula.yml`, which run only on tags,
  `workflow_run` or manual dispatch.
- Changing what any existing job does, its runner, its permissions or its
  triggers.
- Configuring the org ruleset that requires the `Archive` check.
- Archiving or otherwise touching the change workspaces other people opened.

## Steps

1. `ci.yml`: add `archive-gate`; add `needs: archive-gate` to `test`, `lint`,
   `audit`, `spec-check` and `intent-check`. `integration`, `corvid-pet` and
   `attest` already need `test`, so they reach the gate transitively.
2. `test-action.yml`: add `archive-gate`; add `needs: archive-gate` to
   `install`, `refuses-unsafe-install` and `windows-unsupported`.
3. `trust.yml`: add `archive-gate`; add `needs: archive-gate` to `trust`.
4. `pages.yml`: add `archive-gate`; add `needs: archive-gate` to `build`.
   `deploy` already needs `build`.
5. Parse each edited workflow and print every job's `needs` to confirm the graph.

## Risks

- While any change is open on `main`, pushes to `main` run nothing else in these
  workflows, including the Pages deploy and the attestation. That is intended:
  archive first.
- The pin is a commit on spec-sync `main`, not a release. It has to be moved to
  the first release that ships the gate.
- `corvid-pet` keeps `if: ... && always()`. It only posts a PR comment, and when
  the gate fails its inputs read `skipped`, so the summary it posts reports
  failure rather than success.

## Constraints consulted

- AGENTS.md golden rule 4 (check exit codes): every check behind this change was
  run and its exit status read.
- AGENTS.md "Project-specific quality gate": `fledge spec check` is required when
  `src/` or `specs/` change; this change touches neither, and the SpecSync
  checks were run with the version CI pins (6.0.0).
