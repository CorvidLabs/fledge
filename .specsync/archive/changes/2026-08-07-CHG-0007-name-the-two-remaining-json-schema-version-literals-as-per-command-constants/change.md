---
id: CHG-0007-name-the-two-remaining-json-schema-version-literals-as-per-command-constants
state: accepted
type: refactor
base_commit: df9ea9c5fd2b6d553a4334afecfbd04cbe23281b
---

# Name the two remaining --json schema_version literals as per-command constants

## Intent

Name the two remaining --json schema_version literals as per-command constants

## Affected Canonical Specs

- None

## Acceptance Criteria

- cargo test, cargo clippy --all-targets -- -D warnings and cargo fmt --check are green; fledge plugins validate --json and fledge lanes validate --json emit byte-identical output to before the change; PLUGINS_VALIDATE_SCHEMA and LANES_VALIDATE_SCHEMA are defined beside the existing per-command constants and used at their call sites; envelope tests cover resource byte-identity and versioned struct flattening.

## No-spec Rationale

Introduces named per-command schema constants and envelope tests only; emitted JSON bytes are unchanged, so no canonical spec contract moves.
