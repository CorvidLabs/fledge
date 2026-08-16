---
change: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
artifact: docs
---

# Docs

## README.md

New `## GitHub Actions` section, placed right after `## Install` (before
`## Quick start`), leading with the pinned form per the task requirement:

```yaml
- uses: CorvidLabs/fledge@v1
  with:
    version: v1.7.2
```

followed by a short explanation of the pin-vs-latest tradeoff and the
outputs shape.

## CONTRIBUTING.md

New "Moving the `v1` tag" subsection under `## Release Process`, documenting
`git tag -f v1 v<version> && git push origin v1 --force` as a manual
post-release step, since `fledge release` does not do this automatically.

## AGENTS.md

Not updated by this change. `AGENTS.md` documents fledge's own `--json`
command surface for agents; a GitHub Action for installing the binary isn't
part of that surface. If a follow-up wants to fold "how to install fledge in
CI" into the agent-facing doc too, that's an easy additive change, not a
prerequisite for this one.

## No docs-site (`site/src/content/docs/`) change

Out of scope for this change per the task's explicit ask (README section
only). `site/src/content/docs/getting-started/installation.md` currently has
no GitHub Actions section either — worth a follow-up for consistency, but
not bundled here to keep this change's diff to what was asked.
