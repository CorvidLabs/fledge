---
id: decompose-six-god-functions-into-smaller-unit-testable-helpers-with-no-behavior-change
state: draft
type: refactor
base_commit: e0fe31bfd6ac39be1a68979d09c09dac25bae5f6
---

# Decompose six god functions into smaller unit-testable helpers with no behavior change

## Intent

Decompose six god functions into smaller unit-testable helpers with no behavior change

## Affected Canonical Specs

- None

## Acceptance Criteria

- Each of the six functions (spec/commands.rs::new_spec; lanes/community.rs::import_lanes; main.rs::run; plugin/install.rs::install_plugin; plugin/update.rs::update_one_plugin; plugin/update.rs::rebuild_after_fetch) reads as a short orchestration of named private helpers under roughly 80 lines; main.rs::run becomes setup plus a flat dispatch match whose arms only move parsed fields into name-checked option structs and call the handler; all existing tests pass unmodified; clippy with -D warnings and cargo fmt --check stay green; no public signature or CLI behavior or output text changes

## No-spec Rationale

Internal restructuring only: the six functions keep their signatures and behavior, new helpers are private, spec frontmatter file lists and documented exports are unchanged, and the full existing test suite passes unmodified.
