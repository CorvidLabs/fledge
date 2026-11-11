---
change: decompose-six-god-functions-into-smaller-unit-testable-helpers-with-no-behavior-change
artifact: requirements
---

# Requirements

- Each of the six functions reads as a short orchestration of named private
  helpers; the outer body is under roughly 80 lines. The exception is
  `main.rs::run`, which becomes setup plus a flat `dispatch` match: its arms
  only move parsed fields into the handler's options struct by field name, and
  every arm with logic of its own is a helper.
- No observable behavior change: same CLI output, exit codes, error text and
  side effects on every path.
- No public API change: existing signatures stay; new helpers are private.
- The full existing test suite passes unmodified; clippy with `-D warnings` and
  `cargo fmt --check` stay green.
