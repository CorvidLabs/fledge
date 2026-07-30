---
id: CHG-0007-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
state: draft
type: feature
base_commit: 3be72407889deb952d37e70f074dd99f16f24370
---

# Fledge spec lint: two-layer quality gate for the spec itself (issue #429)

## Intent

fledge spec lint: two-layer quality gate for the spec itself (issue #429)

## Affected Canonical Specs

- `spec`
- `main`

## Acceptance Criteria

- fledge spec lint [target] exits 0 on a clean spec tree and non-zero with a structured list of failures otherwise; layer 1 (required sections present and non-empty, no TODO/TBD/FIXME in Purpose or Public API, integer-or-semver version, every files: entry present on disk, an acceptance signal and a rejection signal) always runs offline; layer 2 is a model-graded quality pass gated behind --ai that fails fast with a clear error when no provider can answer; --json emits the spec_lint envelope; cargo test, clippy -D warnings and fmt --check are green

## No-spec Rationale

Not applicable
