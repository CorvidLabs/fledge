---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
module: lanes
---

## ADDED

### REQUIREMENT REQ-lanes-013

`fledge lanes run` and `fledge lanes validate` SHALL surface the shared walker's depth
bound as an ordinary error and exit normally, never terminating on a signal.

Acceptance Criteria
- Against a 1,200-task chain, each command exits with a status code rather than a signal.
- The depth error appears in the command's output.
- `fledge run` behaves identically; all three reach the same walker.
