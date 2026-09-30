---
change: publish-to-crates-io-from-the-release-workflow-and-verify-the-registry
artifact: testing
---

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
