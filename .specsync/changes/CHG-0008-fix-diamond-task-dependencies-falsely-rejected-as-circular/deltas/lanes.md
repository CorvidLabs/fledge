---
change: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
module: lanes
---

## ADDED

### REQUIREMENT REQ-lanes-012

`fledge lanes run` and `fledge lanes validate` SHALL use the same two-set DFS as `fledge run` for task-dep cycle detection.

Acceptance Criteria
- Diamond DAG `a → [b, c]`, `b → d`, `c → d` succeeds for `lanes run` and `lanes validate`.
- Genuine cycle `a → b → a` still fails.
- Cycle errors from the shared helper list the ordered walk.
