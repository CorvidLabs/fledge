---
change: CHG-0006-release-v1-7-2-bump-version-files-and-changelog
artifact: plan
---

# Plan

1. Run `fledge release patch` to bump `Cargo.toml`/`flake.nix`, regenerate `Cargo.lock`,
   and append the `CHANGELOG.md` entry.
2. Register this SpecSync change to cover the touched paths.
3. Verify and accept.
