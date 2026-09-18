---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: docs
---

# Docs

## README.md — `## GitHub Actions`

CHG-0008's section claimed:

> every download is checksum-verified against the release's `.sha256` sidecar

which the warn-and-skip path did not deliver. The docs promised a guarantee
the script did not keep, and the reviewer was right that one of the two had to
move. The guarantee is now real, so the sentence is rewritten to describe it
precisely rather than softened: verification is mandatory, a missing,
unfetchable, or mismatched checksum fails the step, and releases before
`v0.9.1` predate the sidecars and therefore cannot be installed this way.

Two further corrections in the same section:

- `version` accepts a release tag or `latest` and nothing else, stated
  alongside the reason (the value reaches a download URL).
- Windows is described as not supported by the action *yet*, with a pointer to
  the `fledge-windows-x86_64.exe` that every release does publish — replacing
  a platform list that a reader could take as "no Windows binary exists".

## CONTRIBUTING.md

Unchanged. The "Moving the `v1` tag" subsection CHG-0008 added is still
accurate and still the manual post-release step.

## action.yml input descriptions

The `version` input's own description now states that anything other than a
release tag or `latest` is rejected, so the constraint is discoverable from
the action's inputs — where a consumer writing `with:` will actually look —
and not only from the README.

## AGENTS.md and the docs site

Not updated, unchanged from CHG-0008's reasoning: `AGENTS.md` documents
fledge's `--json` command surface, and
`site/src/content/docs/getting-started/installation.md` has no GitHub Actions
section to correct. Folding "how to install fledge in CI" into either remains
an additive follow-up.
