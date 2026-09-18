---
change: fix-diamond-task-dependencies-falsely-rejected-as-circular
artifact: testing
---

# Testing

## REQ-run-014: two-set DFS; diamond is not a cycle; ordered cycle walk

- Automated: `src/deps.rs::tests::diamond_is_not_a_cycle_and_shared_dep_runs_once`
- Automated: `src/deps.rs::tests::two_cycle_reports_ordered_path`
- Automated: `src/run.rs::tests::diamond_deps_are_not_circular`
- Automated: `src/run.rs::tests::detect_circular_deps`
- Automated: `tests/run.rs::cli_run_diamond_deps_succeeds`

## REQ-lanes-012: lanes run and validate share the same two-set DFS

- Automated: `src/lanes/tests.rs::execute_diamond_deps_are_not_circular`
- Automated: `src/lanes/tests.rs::execute_real_cycle_is_detected`
- Automated: `src/lanes/tests.rs::validate_lanes_diamond_deps_ok`
- Automated: `src/lanes/tests.rs::validate_lanes_real_cycle_fails`
- Automated: `tests/lanes.rs::cli_lane_run_diamond_deps_succeeds`
- Automated: `tests/lanes.rs::cli_lane_validate_diamond_deps_succeeds`
- Automated: `tests/lanes.rs::cli_lane_run_real_cycle_fails`

## REQ-main-011: the crate root declares the shared walk module

- Code (`src/main.rs:12`): `mod deps;` — the declaration that makes one
  implementation of the walk resolvable crate-wide.
- Code: all three consumers reach it through that declaration and none keeps a
  second walk — `src/run.rs`, `src/lanes/execute.rs` and `src/lanes/validate.rs`
  each call `crate::deps::walk_task_graph`, verified by
  `grep -rn "crate::deps::walk_task_graph" src/` returning exactly those three.
- Automated: the acceptance criterion "`cargo build` resolves `crate::deps` from
  every one of those call sites" is enforced by compilation itself — the whole
  suite builds and passes, which is impossible if the declaration or any call
  path is missing. No separate assertion can be stronger than the compiler here.

## Rejection signal

If a diamond DAG (`a → [b, c]`, `b → d`, `c → d`) is reported as circular, or a
genuine `a → b → a` cycle is accepted, the change is wrong.
