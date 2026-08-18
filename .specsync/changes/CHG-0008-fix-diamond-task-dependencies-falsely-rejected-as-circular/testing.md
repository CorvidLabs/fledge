---
change: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
artifact: testing
---

# Testing

## REQ-run-008: two-set DFS; diamond is not a cycle; ordered cycle walk

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

## Rejection signal

If a diamond DAG (`a → [b, c]`, `b → d`, `c → d`) is reported as circular, or a
genuine `a → b → a` cycle is accepted, the change is wrong.
