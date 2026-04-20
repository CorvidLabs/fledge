---
module: changelog
type: context
---

# Changelog Context

## Background

Fledge already tags releases (v0.3.0 through v0.7.0) and uses conventional commit prefixes. A changelog command turns this existing practice into a useful output without requiring any additional metadata files or configuration.

## Assumptions

- The project uses git tags for releases
- Tags follow semver (optionally prefixed with `v`)
- Commits follow conventional commit format (feat:, fix:, docs:, etc.)
- Non-conventional commits are still included under "Other"

## Design Decisions

- Uses git CLI directly rather than a git library to keep dependencies minimal
- Groups by BTreeMap for alphabetical section ordering
- Excludes merge commits since they duplicate information
