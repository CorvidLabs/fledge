---
change: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
artifact: tasks
---

# Tasks

- [x] Add `src/deps.rs` two-set DFS (`walk_task_graph`)
- [x] Wire `run.rs` execute path through the helper
- [x] Wire `lanes/execute.rs` through the helper; delete the one-set recursive walk
- [x] Replace `lanes/validate.rs` pop-stack walk with the recursive helper
- [x] Report cycle errors as an ordered walk
- [x] Add diamond + genuine-cycle tests at the helper, run, execute, and validate sites
- [x] Sync run and lanes specs
- [x] `cargo test` and `cargo clippy --all-targets` green
