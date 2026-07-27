---
change: CHG-0006-release-v1-7-2-bump-version-files-and-changelog
artifact: context
---

# Context

`fledge release patch` bumps `Cargo.toml`/`flake.nix` to the new version, regenerates
`Cargo.lock`, and appends a `CHANGELOG.md` entry. This release includes the
`parse_source_ref` slash-ref fix (#501). Routine release mechanics with no runtime
behavior change; this change exists purely to give the release commit SpecSync coverage
(same pattern as CHG-0004 for v1.7.1).
