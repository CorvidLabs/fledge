---
change: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
module: run
---

## ADDED

### REQUIREMENT REQ-run-008

Task dependency walks SHALL use a shared two-set DFS (`in_progress` vs `completed`) so a completed node on another branch is skipped rather than treated as a back edge. Circular dependencies SHALL produce an error listing the ordered cycle walk. A diamond DAG (two tasks sharing one dep) is not a cycle.

Acceptance Criteria
- `fledge run a` on `a → [b, c]`, `b → d`, `c → d` succeeds and runs `d` once.
- `fledge run a` on `a → b → a` fails with `Circular dependency detected: a → b → a`.
- `src/deps.rs` `walk_task_graph` is the helper used by `fledge run` execution.
