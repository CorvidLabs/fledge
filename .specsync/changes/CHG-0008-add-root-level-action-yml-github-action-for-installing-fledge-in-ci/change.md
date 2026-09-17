---
id: CHG-0008-add-root-level-action-yml-github-action-for-installing-fledge-in-ci
state: accepted
type: feature
base_commit: 6eb7a3ea595c949f4bc0078c0738508d800356b3
---

# Add root-level action.yml GitHub Action for installing fledge in CI

## Intent

Add root-level action.yml GitHub Action for installing fledge in CI

## Affected Canonical Specs

- None

## Acceptance Criteria

- uses: CorvidLabs/fledge@v1 with version: v1.7.2 installs fledge on Linux and macOS runners and makes zero GitHub API calls; version: latest resolves via an authenticated API call; a corrupted download fails the checksum-verification step; Windows runners fail with a readable error instead of a 404 mid-download; the new exercising workflow passes on ubuntu-latest and macos-latest for both a pinned tag and latest; fledge lanes run check passes.

## No-spec Rationale

No canonical spec module governs this: .specsync/config.toml scans only source_dirs [src, templates], and the closest existing specs (specs/release for the fledge release command internals, specs/github for fledge's own GitHub API helper module) do not cover the repo's own CI/distribution surface. Only action.yml, a new workflow file, README.md, and CONTRIBUTING.md change; no src/ or templates/ files.
