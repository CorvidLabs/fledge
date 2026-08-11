---
change: CHG-0007-name-the-two-remaining-json-schema-version-literals-as-per-command-constants
artifact: context
---

# Context

Issue #444 called for a shared typed `--json` envelope builder to stop shape drift
between the two documented dialects. An audit of the current tree found the migration
already complete: all 54 outer envelopes route through
`crate::envelope::{resource,action,versioned}`, and the only `"schema_version"`
literals left outside `src/envelope.rs` are four test expectations in
`src/publish.rs` that deliberately assert byte-identity against
`build_publish_envelope`.

Two call sites remained inconsistent with their own modules. `src/plugin/validate.rs`
and `src/lanes/validate.rs` passed a bare `1` for `schema_version` while every
sibling envelope in those modules reads a named `*_SCHEMA` constant from the module's
constants block (nine such constants in `src/plugin/mod.rs`, eight in
`src/lanes/mod.rs`).

That inconsistency is the precise drift risk #444 was filed against: a future shape
change to either `validate` command would require grepping for a magic number rather
than editing the constants block alongside its peers, which is how divergence gets
introduced silently.

Two test gaps in `src/envelope.rs` are closed at the same time, because they cover the
exact guarantee this refactor relies on. `resource` had no byte-identity test even
though `action` and `versioned` both did, and nothing covered the
`versioned(v, to_value(struct))` shape both validate commands use, where the struct's
fields must flatten to the envelope's top level rather than nest under a key.
