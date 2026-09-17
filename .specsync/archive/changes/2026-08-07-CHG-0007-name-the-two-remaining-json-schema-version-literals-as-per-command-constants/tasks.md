---
change: CHG-0007-name-the-two-remaining-json-schema-version-literals-as-per-command-constants
artifact: tasks
---

# Tasks

- [x] Add `PLUGINS_VALIDATE_SCHEMA` to `src/plugin/mod.rs`
- [x] Add `LANES_VALIDATE_SCHEMA` to `src/lanes/mod.rs`
- [x] Use `PLUGINS_VALIDATE_SCHEMA` in `src/plugin/validate.rs`
- [x] Use `LANES_VALIDATE_SCHEMA` in `src/lanes/validate.rs`
- [x] Add `resource` byte-identity test to `src/envelope.rs`
- [x] Add `versioned` struct-flattening test to `src/envelope.rs`
- [x] Verify emitted JSON is unchanged for both validate commands
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` green
