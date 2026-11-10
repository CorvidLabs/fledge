---
change: decompose-six-god-functions-into-smaller-unit-testable-helpers-with-no-behavior-change
artifact: testing
---

# Testing

The regression net is the existing suite, unchanged: `fledge lanes run
pre-commit` (fmt + clippy `-D warnings` + full tests + spec check). Focused
modules: `tests/main.rs` (dispatch), `tests/isolation.rs`, plus the
plugin/lanes/spec integration tests. Any behavior difference shows up as an
existing-test failure; none of the test files are edited.
