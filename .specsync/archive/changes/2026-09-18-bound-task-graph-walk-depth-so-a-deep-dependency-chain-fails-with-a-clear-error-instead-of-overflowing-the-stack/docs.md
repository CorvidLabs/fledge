---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
artifact: docs
---

# Docs

The bound is a new failure mode for three documented commands, so it is recorded as an
invariant in both affected canonical specs rather than only in code:

- `specs/run/run.spec.md` — invariant 20 (the `run` spec owns `src/deps.rs`), Change Log
  version 10.
- `specs/lanes/lanes.spec.md` — invariant 9 extended to name the bound for `lanes run`
  and `lanes validate`, Change Log version 27.

No user-facing documentation change beyond the specs. `MAX_TASK_DEPTH` is not
configurable, so there is no new flag, field or `fledge.toml` key to document, and the
existing `--help` text for `run`, `lanes run` and `lanes validate` is unaffected.

The error text is self-explaining at the point of failure:

```text
Dependency chain deeper than 1000 tasks (reached 't1000') — check task deps for an unintended chain
```
