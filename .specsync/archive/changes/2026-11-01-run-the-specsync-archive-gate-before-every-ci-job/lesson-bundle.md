# Lesson bundle — run-the-specsync-archive-gate-before-every-ci-job

Material for folding this change's lessons into the affected specs' `context.md`.
Synthesise from what actually happened below; do not restate the change description.

## What this change was

- **Title**: Run the SpecSync archive gate before every CI job
- **Kind**: Operations
- **Paths**: .github/workflows/ci.yml, .github/workflows/pages.yml, .github/workflows/test-action.yml, .github/workflows/trust.yml
- **Acceptance**: Every workflow under .github/workflows that runs on pull_request or push (ci.yml, test-action.yml, trust.yml, pages.yml) starts with an archive-gate job named Archive that runs CorvidLabs/spec-sync pinned to 106aaa43d0bcf049f19b173ddb63815a25c3f931 with archive-gate-only: 'true'; every other job in those workflows needs archive-gate directly or transitively, so nothing else runs while a change under .specsync/changes/ is still open.

## Evidence

- Verification commit: `16ccb58608642b7c91b933483e64ba7d3b13be01`
- Base commit: `b648ea2a622221d4f2ffbc7ba7aa829e6cf13a51`
- Verified by: `specsync check (no spec in scope)`

## From the change's context.md

# Context

CorvidLabs is rolling out the SpecSync archive gate across the organization
(CorvidLabs/spec-sync#832, merged as 106aaa43d0bcf049f19b173ddb63815a25c3f931;
rollout approved by 0xLeif). The rule: every SpecSync change must be archived
before a pull request merges, and CI must run nothing until it is. Until now
fledge's CI ran its full Rust, spec and action suites while a change workspace
under `.specsync/changes/` was still open, so an unarchived change could reach
`main` with green checks.

Things a session picking this up needs to know:

- The `archive-gate-only` input only exists on spec-sync `main` at
  106aaa43d0bcf049f19b173ddb63815a25c3f931. Published v6.1.1 and older ignore
  it, so the step pins that commit instead of a tag. Move it to the first
  release that ships the gate.
- The gate runs `.github/scripts/archive-readiness.sh` from the action. It needs
  only bash and python3, so it runs on `ubuntu-latest` and downloads no binary.
- A job skipped because its dependency failed counts as passing for branch
  protection. Blocking merges therefore relies on an org ruleset that requires a
  check named `Archive`; that ruleset is configured separately.
- `release.yml` (tag pushes only) and `post-release-formula.yml`
  (`workflow_run` and `workflow_dispatch` only) are left alone: they never run on
  a pull request or a branch push.
- The change `upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314` was
  already open on `main` when this work started. It belongs to someone else and
  is not touched here; the new `Archive` check stays red until it is archived.

## From the change's testing.md

# Testing

- Each edited workflow parses as YAML
  (`ruby -e 'require "psych"; Psych.safe_load(File.read(ARGV[0]))'`).
- Printing every job's `needs` shows `archive-gate` with no dependencies and
  every other job reaching it directly or through `test` / `build`.
- `bash .github/scripts/archive-readiness.sh .` from CorvidLabs/spec-sync run
  against this checkout fails while any change under `.specsync/changes/` is
  open, which is what the `Archive` job will do on the pull request until this
  change and the pre-existing open change are archived.
- `specsync change audit --strict` (6.0.0, the version CI pins) reports the
  workflow paths covered by this change.
- On the pull request, the `Archive` check appears first in each workflow, and
  no other job starts until it passes.

## Where these lessons go

This change declared no affected specs, so there is no module context to fold into.
