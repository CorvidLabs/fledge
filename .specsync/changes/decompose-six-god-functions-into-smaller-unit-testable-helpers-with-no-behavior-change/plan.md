---
change: decompose-six-god-functions-into-smaller-unit-testable-helpers-with-no-behavior-change
artifact: plan
---

# Plan

## Approach

Per function: read the body, identify the natural phases (validation, fetch,
decision, execution, reporting), extract each phase into a private helper with a
name that says what it returns, and leave the outer function as the orchestrator.
Extract verbatim logic — no behavior edits while moving code.

## Out of scope

No public API or signature changes, no output-text changes, no new files, no new
tests (the existing suite is the regression net), no spec changes.

## Steps

1. `main.rs::run`, then focused tests.
2. `plugin/install.rs::install_plugin`, then `plugin/update.rs::update_one_plugin` and `rebuild_after_fetch` (shared idioms), then plugin tests.
3. `lanes/community.rs::import_lanes`, then lanes tests.
4. `spec/commands.rs::new_spec`, then spec tests.
5. Full `fledge lanes run pre-commit` (fmt + clippy `-D warnings` + tests + spec check), then PR as ready-for-review for Leif.

## Risks

Extraction slips that alter behavior — caught by the unchanged existing suite. Long
parameter lists on helpers — kept readable by grouping into small structs only where
a phase genuinely shares state.

## Constraints consulted

Issue #446 scope; repo rule that new requirements/features stop at a ready PR;
AGENTS.md source map for module ownership; the change's own no-spec rationale.
