---
id: run-the-specsync-archive-gate-before-every-ci-job
state: implementing
type: operations
base_commit: b648ea2a622221d4f2ffbc7ba7aa829e6cf13a51
---

# Run the SpecSync archive gate before every CI job

## Intent

Run the SpecSync archive gate before every CI job

## Affected Canonical Specs

- None

## Acceptance Criteria

- Every workflow under .github/workflows that runs on pull_request or push (ci.yml, test-action.yml, trust.yml, pages.yml) starts with an archive-gate job named Archive that runs CorvidLabs/spec-sync pinned to 106aaa43d0bcf049f19b173ddb63815a25c3f931 with archive-gate-only: 'true'; every other job in those workflows needs archive-gate directly or transitively, so nothing else runs while a change under .specsync/changes/ is still open.

## No-spec Rationale

CI workflow ordering only; no canonical spec owns .github/workflows
