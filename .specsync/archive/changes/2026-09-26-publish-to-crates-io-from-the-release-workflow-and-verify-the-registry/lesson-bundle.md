# Lesson bundle — publish-to-crates-io-from-the-release-workflow-and-verify-the-registry

Material for folding this change's lessons into the affected specs' `context.md`.
Synthesise from what actually happened below; do not restate the change description.

## What this change was

- **Title**: Publish to crates.io from the release workflow and verify the registry
- **Kind**: BugFix
- **Paths**: .github/workflows/release.yml, AGENTS.md
- **Acceptance**: release.yml carries a publish job that runs cargo publish on a v* tag after the release job, fails loudly with an actionable message when CARGO_REGISTRY_TOKEN is absent rather than skipping, and then reads https://crates.io/api/v1/crates/fledge back and compares max_version to the tag, retrying while the index catches up; AGENTS.md documents both the job and the read-the-registry-back rule; the workflow remains valid YAML with jobs test, build, release, publish

## Evidence

- Verification commit: `bf374c9c51030ef51e8314a8d7cb2e02e34bff75`
- Base commit: `31d5f45f7bca1b71e374e41792fabeb6359a9de4`
- Verified by: `specsync check (no spec in scope)`

## From the change's context.md

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

## From the change's testing.md

# Testing

## What was verified

| Check | Result |
|---|---|
| `release.yml` parses as YAML | passes; jobs are `test`, `build`, `release`, `publish` |
| `publish.needs` | `[release]` — publishes only after the GitHub release exists |
| Trigger unchanged | still `on: push: tags: v*`; no new trigger surface |
| No publish step existed before | `grep -rn "cargo publish" .github/workflows/` returned nothing |
| The gap is real | crates.io `max_version` = 1.7.0 while `v1.7.1` and `v1.7.2` are tagged |
| Prior runs did not fail | the Release workflow is green for both tags in the Actions history |

## What cannot be verified here

The job itself cannot run until a `v*` tag is pushed **and** `CARGO_REGISTRY_TOKEN`
exists. Both halves are untestable in a PR:

- Without the secret, the first release after this merges will fail at the publish job
  with the actionable message. That is the designed behaviour, not a regression — but it
  does mean the first real exercise of this code is a release.
- The registry read-back loop is only exercised on a genuine publish.

This is an honest limitation. The mitigation is that the failure mode is loud and its
message names the exact fix, rather than silent.

## Rejection signals

- A tagged release whose workflow is green while crates.io still reports the previous
  version — the original bug, and what this job exists to make impossible.
- The publish job skipping (rather than failing) when the token is absent.
- `cargo publish` succeeding and the job passing without the registry confirming the new
  version.

## Where these lessons go

This change declared no affected specs, so there is no module context to fold into.
