---
id: prepare-fledge-1-8-1-release-closeout-with-the-security-prerequisite-held
state: draft
type: documentation
base_commit: 85d7c4efbca79947afa359504b095c95c486b391
---

# Prepare Fledge 1.8.1 release closeout with the security prerequisite held

## Intent

Prepare Fledge 1.8.1 release closeout with the security prerequisite held

## Affected Canonical Specs

- None

## Acceptance Criteria

- The 1.8.1 closeout states the current channel mismatch, records that the Wasmtime security upgrade (#538) and zstd cleanup (#543) are merged with a green hosted audit, lists #540 and #541 among the release contents, and holds every publication step (tag, GitHub release, crates.io, Homebrew) for explicit owner authorization. No runtime, dependency or workflow edits.

## No-spec Rationale

Release handoff and changelog qualification only; package and Nix already declare 1.8.1. No runtime or dependency edits.

## Update 2026-11-10

The security prerequisite this change was drafted to hold on (#538) has since merged, together with #543, #540 and #541. The handoff document and changelog were rebased onto that main and rewritten to describe the resolved hold; publication remains unauthorized. The change stays in draft: definition approval, review and finalization belong to the owner who decides the release.
