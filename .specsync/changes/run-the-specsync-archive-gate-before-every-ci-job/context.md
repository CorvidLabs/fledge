---
change: run-the-specsync-archive-gate-before-every-ci-job
artifact: context
---

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
