---
change: fix-diamond-task-dependencies-falsely-rejected-as-circular
module: main
---

## ADDED

### REQUIREMENT REQ-main-011

The crate root SHALL declare the shared task-dependency walk module (`mod deps;`) so `fledge run`, `fledge lanes run`, and `fledge lanes validate` resolve one implementation of the walk instead of each carrying its own.

Acceptance Criteria
- `src/main.rs` declares `mod deps;`.
- `src/run.rs`, `src/lanes/execute.rs`, and `src/lanes/validate.rs` all reach `walk_task_graph` through that declaration; no module keeps a second dependency walk.
- `cargo build` resolves `crate::deps` from every one of those call sites.
