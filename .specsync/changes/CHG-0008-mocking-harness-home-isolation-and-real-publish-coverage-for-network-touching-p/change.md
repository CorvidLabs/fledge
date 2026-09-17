---
id: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
state: accepted
type: refactor
base_commit: 47cbefdc5e4cb1c6d7e3a3075deb5d88ae576c84
---

# Mocking harness, HOME isolation and real publish coverage for network-touching paths

## Intent

Mocking harness, HOME isolation and real publish coverage for network-touching paths

## Affected Canonical Specs

- `publish`
- `github`
- `llm`
- `doctor`

## Acceptance Criteria

- cargo test, cargo clippy --all-targets -- -D warnings and cargo fmt --check are green on Linux, macOS and Windows; no test performs real network I/O or reads the developer's real HOME or ~/.config/fledge; src/publish.rs has real coverage replacing the empty #[ignore] stub, including a push path asserted via git remote get-url rather than raw OS-path substring matching; production behavior is unchanged and release builds keep the production GitHub API and remote base constants.

## No-spec Rationale

Not applicable
