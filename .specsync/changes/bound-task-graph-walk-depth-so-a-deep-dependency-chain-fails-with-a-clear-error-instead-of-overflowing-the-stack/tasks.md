---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
artifact: tasks
---

# Tasks

- [x] Add `MAX_TASK_DEPTH` and the depth guard to `deps::walk_task_graph`, placed after
      the cycle check so cycles keep their more specific error
- [x] Unit tests: at the bound, one past it, 20,000 deep, and wide-but-shallow
- [x] Verify the 20,000 test fails without the guard (reproduced the SIGABRT), then restore
- [x] Confirm all three callers reach the same walker — `src/run.rs:558`,
      `src/lanes/execute.rs:382`, `src/lanes/validate.rs:126` — and cover each
- [x] Add `deep_chain_toml` helper to `tests/common/mod.rs`
- [x] Document the bound as an invariant in the `run` spec (owner of `src/deps.rs`) and
      in the `lanes` spec, with Change Log entries and version bumps
- [x] `fledge lanes run pre-commit` (green 3 consecutive runs), `fledge spec check` (33 specs, 0/0), `fledge spec lint run`/`lanes` (0/0)
- [x] Definition approval recorded by the owner (user:0xLeif)
