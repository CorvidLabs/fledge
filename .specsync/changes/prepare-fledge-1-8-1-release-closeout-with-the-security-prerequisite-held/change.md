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

- The 1.8.1 closeout states the current channel mismatch and holds publication on PR 538 without altering runtime dependencies or claiming the security upgrade is merged.

## No-spec Rationale

Release handoff and changelog qualification only; package and Nix already declare 1.8.1. No runtime or dependency edits and no claim that pending PR 538 is merged.
