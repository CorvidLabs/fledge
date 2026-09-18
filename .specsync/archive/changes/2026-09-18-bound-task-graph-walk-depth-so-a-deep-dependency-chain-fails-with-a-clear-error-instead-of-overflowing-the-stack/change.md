---
id: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
state: archived
type: bug_fix
base_commit: f659d69908c607295f2103cdb23e48c09a610965
---

# Bound task-graph walk depth so a deep dependency chain fails with a clear error instead of overflowing the stack

## Intent

Bound task-graph walk depth so a deep dependency chain fails with a clear error instead of overflowing the stack

## Affected Canonical Specs

- `run`
- `lanes`

## Acceptance Criteria

- walk_task_graph bails with a clear, catchable error naming the depth bound once a dependency chain exceeds MAX_TASK_DEPTH, instead of aborting the process with a stack overflow; a chain of exactly MAX_TASK_DEPTH still walks successfully; fledge run <task>, fledge lanes run <lane> and fledge lanes validate all surface that error rather than exit 134; the diamond-DAG behaviour and ordered cycle-path reporting from #513 are unchanged; cargo test, clippy -D warnings and fmt --check are green

## No-spec Rationale

Not applicable
