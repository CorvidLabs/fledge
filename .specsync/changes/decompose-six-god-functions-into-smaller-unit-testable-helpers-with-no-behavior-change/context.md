---
change: decompose-six-god-functions-into-smaller-unit-testable-helpers-with-no-behavior-change
artifact: context
---

# Context

Issue #446 (filed from a staff-level review): six production functions concentrate
the pillar logic that is hardest to test, at roughly 190–220 lines each on current
main (the issue's 210–480 counts predate later edits):

- `src/spec/commands.rs::new_spec` (218 lines)
- `src/lanes/community.rs::import_lanes` (214)
- `src/main.rs::run` (211)
- `src/plugin/install.rs::install_plugin` (208)
- `src/plugin/update.rs::update_one_plugin` (197)
- `src/plugin/update.rs::rebuild_after_fetch` (191)

Constraints: no behavior change, no public-signature change, no spec change
(helpers stay private, so export documentation and frontmatter file lists are
untouched). The existing test suite is the regression net and must pass
unmodified. Leif authorized this as far as a ready PR (2026-11-10, relayed by the
coordinator); definition approval and merge are Leif's review, not the worker's.

Ruled out: positional option-builder helpers for `main.rs` (e.g. a
nine-argument `release_options(bump, dry_run, no_tag, ...)`). They trade the
name-checked struct-literal fields for positional same-typed bools that
compile silently when swapped, and need `#[allow(clippy::too_many_arguments)]`;
the dispatch arms keep their struct literals instead. Also ruled out: changing any observable output text while moving code (keeps the
diff reviewable and the tests untouched); splitting into new files (same-file
private helpers keep the blast radius small).
