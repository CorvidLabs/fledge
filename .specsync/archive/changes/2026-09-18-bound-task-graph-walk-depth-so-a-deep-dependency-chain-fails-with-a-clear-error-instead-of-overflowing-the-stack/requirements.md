---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
artifact: requirements
---

# Requirements

### REQ-run-023

`deps::walk_task_graph` SHALL descend at most `MAX_TASK_DEPTH` levels of dependency
nesting and SHALL return an `Err` naming that bound when a graph is deeper, rather than
exhausting the thread stack and aborting the process.

**Acceptance**
- A chain of exactly `MAX_TASK_DEPTH` tasks walks successfully.
- A chain of `MAX_TASK_DEPTH + 1` returns an error containing `Dependency chain deeper than`.
- A chain of 20,000 returns that same error; before the bound this aborted the test
  binary with `fatal runtime error: stack overflow`.

### REQ-run-024

The bound SHALL apply to nesting depth only, so that a graph with more than
`MAX_TASK_DEPTH` tasks but shallow nesting still walks.

**Acceptance**
- A root with 1,999 leaf dependencies (2,000 tasks, 2 levels) walks successfully.

### REQ-lanes-013

`fledge run <task>`, `fledge lanes run <lane>` and `fledge lanes validate` SHALL each
surface the depth error and exit normally, never terminating on a signal.

**Acceptance**
- For each of the three commands against a 1,200-task chain: the process exits with a
  status code (not a signal), and the depth error appears in its output.
