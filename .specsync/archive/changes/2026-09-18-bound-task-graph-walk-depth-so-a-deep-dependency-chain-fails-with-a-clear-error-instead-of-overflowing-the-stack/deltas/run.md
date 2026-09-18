---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
module: run
---

## ADDED

### REQUIREMENT REQ-run-023

The shared task-graph walk SHALL be depth-bounded, failing with a catchable error that
names the bound rather than exhausting the thread stack and aborting the process.

Acceptance Criteria
- A dependency chain of exactly `MAX_TASK_DEPTH` tasks walks successfully.
- A chain one past the bound returns an error containing `Dependency chain deeper than`.
- A chain of 20,000 returns that same error instead of `fatal runtime error: stack overflow`.
- A cycle found deeper than the bound is still reported as a cycle; the cycle check runs first.

### REQUIREMENT REQ-run-024

The bound SHALL apply to dependency *nesting*, not to the number of tasks.

Acceptance Criteria
- A root with 1,999 leaf dependencies (2,000 tasks, 2 levels deep) walks successfully.
