---
change: CHG-0007-name-the-two-remaining-json-schema-version-literals-as-per-command-constants
artifact: plan
---

# Plan

1. Declare `PLUGINS_VALIDATE_SCHEMA` in `src/plugin/mod.rs` beside the existing
   per-command schema constants.
2. Declare `LANES_VALIDATE_SCHEMA` in `src/lanes/mod.rs` beside its existing
   per-command schema constants.
3. Replace the bare `1` at the `src/plugin/validate.rs` envelope call site with
   `PLUGINS_VALIDATE_SCHEMA`.
4. Replace the bare `1` at the `src/lanes/validate.rs` envelope call site with
   `LANES_VALIDATE_SCHEMA`.
5. Add the `resource` byte-identity and `versioned` struct-flattening tests to
   `src/envelope.rs`.
6. Confirm `fledge plugins validate --json` and `fledge lanes validate --json` emit
   output identical to the pre-change binary.
7. Run the full gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test`.
