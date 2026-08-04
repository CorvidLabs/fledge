---
change: CHG-0004-release-v1-7-1-bump-version-files-and-changelog
artifact: context
---

# Context

`fledge release patch` bumps `Cargo.toml`/`flake.nix` to the new version, regenerates
`Cargo.lock`, and appends a `CHANGELOG.md` entry. This is routine release mechanics with
no runtime behavior change, but two of the touched paths (`Cargo.lock`, `flake.nix`)
aren't covered by any existing accepted change, and CHG-0003's evidence exact-pins
`Cargo.lock`. This change exists purely to give the release commit SpecSync coverage.
