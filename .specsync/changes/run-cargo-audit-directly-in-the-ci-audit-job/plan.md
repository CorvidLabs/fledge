---
change: run-cargo-audit-directly-in-the-ci-audit-job
artifact: plan
---

# Plan

## Approach

Run cargo-audit directly in the `audit` job, as in design.md.

## Out of scope

The wasmtime bump (CorvidLabs/fledge#548), the advisories themselves, adding `--deny`
flags or an `audit.toml`, and the other CI jobs.

## Steps

1. Edit `.github/workflows/ci.yml`.
2. Run `cargo generate-lockfile && cargo audit` locally, and run `specsync check`.
3. Open the PR as a draft, and leave approval and review to Leif.

## Risks

- `cargo install cargo-audit --locked` compiles cargo-audit each run (about 3 minutes). The
  action did the same.
- cargo-audit's latest release could change its exit codes. It is pinned only by `--locked`
  dependencies, the same as the action, which also installs the latest release.

## Constraints consulted

The note on issue #451 in ci.yml: `audit` is a required status check, so the job must still
run on every pull request and keep its name.
