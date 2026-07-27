---
change: CHG-0006-release-v1-7-2-bump-version-files-and-changelog
artifact: design
---

# Design

No design decisions: `fledge release patch` handles the mechanics (version bump,
lockfile regeneration, changelog generation) per its existing implementation in
`src/release/`. This change only registers those already-produced file changes
with SpecSync so the delivery diff has coverage.
