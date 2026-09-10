---
change: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
artifact: context
---

# Context

Issue #508: a diamond task graph (two tasks sharing one dependency) is reported
as a circular dependency and hard-fails. The repro is `a → [b, c]`, `b → d`,
`c → d` — a DAG. `fledge run a`, `fledge lanes run build`, and
`fledge lanes validate` all fail; `fledge lanes run build --dry-run` passes
because it uses the one correct detector (`check_dep_cycle` in `src/lanes/mod.rs`).

Three DFS implementations used a single accumulating `visited` set and never
popped, so they could not tell "already completed on another branch" from
"currently on the recursion stack":

- `src/run.rs` `execute_task`
- `src/lanes/execute.rs` `execute_task_recursive`
- `src/lanes/validate.rs` iterative pop-stack walk

The cycle error in execute also printed a `HashSet` iteration order, which is
not a walk in the graph.
