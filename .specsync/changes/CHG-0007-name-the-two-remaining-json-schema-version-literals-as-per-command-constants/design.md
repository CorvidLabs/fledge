---
change: CHG-0007-name-the-two-remaining-json-schema-version-literals-as-per-command-constants
artifact: design
---

# Design

## Approach

Add one constant per module beside the existing per-command constants, then read it at
the call site.

- `src/plugin/mod.rs` gains `PLUGINS_VALIDATE_SCHEMA`, joining the nine constants
  already declared there.
- `src/lanes/mod.rs` gains `LANES_VALIDATE_SCHEMA`, joining the eight already there.
- `src/plugin/validate.rs` and `src/lanes/validate.rs` swap their bare `1` for the
  new constant.

Both constants are `1`, matching the value the call sites already passed.

## Wire-format invariant

Emitted JSON is unchanged, byte for byte. This is a readability and
future-maintainability change only; no consumer can observe it. Each command keeps its
own independently versioned schema, per the envelope contract in AGENTS.md — sharing the
value `1` does not couple the two commands' versions.

## Test additions

`src/envelope.rs` gains two tests:

1. `resource` byte-identity against a hand-rolled `json!`, matching the coverage
   `action` and `versioned` already have.
2. `versioned` applied to a serialized struct, asserting the struct's fields land at
   the envelope's top level next to `schema_version` rather than nested.

## Alternatives considered

Leaving the literals in place was rejected: it preserves exactly the grep-for-a-magic-number
hazard #444 exists to remove, and the two files would remain the only envelopes in their
modules not following the local convention.
