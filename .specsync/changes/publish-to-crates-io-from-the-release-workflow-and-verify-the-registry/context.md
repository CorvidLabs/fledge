---
change: publish-to-crates-io-from-the-release-workflow-and-verify-the-registry
artifact: context
---

# Context

## What led here

crates.io has fledge at **1.7.0**, but **v1.7.1 and v1.7.2 are both tagged** on GitHub.
Two releases were cut and never shipped, and nobody noticed.

The cause is not a failure. `release.yml` ran for both tags and **succeeded** — its runs
are green in the Actions history. It has three jobs: `test`, `build`, `release`. None of
them publishes. `grep -rn "cargo publish" .github/workflows/` returns nothing, and
publishing was documented nowhere: not in `AGENTS.md`, not in `CONTRIBUTING.md`, not in
`docs/`.

So publishing was a manual step that existed only in somebody's head, and the workflow
went green while the release was half-done. That is the worst shape a release process can
have: the signal you would check says everything is fine.

## Decisions

**Automate rather than document-only.** A documented manual step is the thing that
already failed twice. The job runs on the same `v*` tag push as the rest of the release.

**Fail loudly when the token is missing, do not skip.** The repository currently has no
secrets at all, so `CARGO_REGISTRY_TOKEN` must be created before the next release. Until
then this job fails, and the release shows red. That is deliberate and it is the whole
point: a release that did not publish is not a finished release, and a skipped step is
indistinguishable from a passing one — which is precisely how 1.7.1 and 1.7.2 slipped by.

**Read the registry back.** `cargo publish` exiting 0 does not prove the crate is live:
the index takes a moment to update, and some no-op paths also exit 0. The job polls
`https://crates.io/api/v1/crates/fledge` and compares `max_version` against the tag,
retrying up to six times. CorvidLabs/hi hit this exact failure at 0.2.4/0.2.5 and its
own guidance now carries the same rule.

## Open ends

- **The secret does not exist yet.** `gh secret list` is empty. Someone with repo admin
  must add `CARGO_REGISTRY_TOKEN` or the next tag will fail at this job.
- **1.7.1 and 1.7.2 remain unpublished.** This change stops the bleeding; it does not
  backfill. Publishing them retroactively is a separate decision — the versions are
  tagged, so they can be published from those tags if wanted, or skipped in favour of
  1.8.0.
