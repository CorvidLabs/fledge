---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
artifact: testing
---

# Testing

## Requirement-to-test evidence

| Requirement | Test | Location |
|---|---|---|
| REQ-run-023 | `chain_at_the_depth_limit_walks` | `src/deps.rs` |
| REQ-run-023 | `chain_one_past_the_depth_limit_bails` | `src/deps.rs` |
| REQ-run-023 | `deep_chain_bails_instead_of_overflowing_the_stack` | `src/deps.rs` |
| REQ-run-024 | `depth_is_nesting_not_total_nodes` | `src/deps.rs` |
| REQ-lanes-013 | `cli_run_deep_chain_fails_cleanly` | `tests/run.rs` |
| REQ-lanes-013 | `cli_lane_run_deep_chain_fails_cleanly` | `tests/lanes.rs` |
| REQ-lanes-013 | `cli_lane_validate_deep_chain_fails_cleanly` | `tests/lanes.rs` |

## The regression test actually fails without the fix

`deep_chain_bails_instead_of_overflowing_the_stack` is the test #513's plan claimed.
It was verified by removing the guard and re-running, which reproduced the reported
crash exactly:

```
thread 'deps::tests::deep_chain_bails_instead_of_overflowing_the_stack' has overflowed its stack
fatal runtime error: stack overflow, aborting
process didn't exit successfully: ... (signal: 6, SIGABRT: process abort signal)
```

The guard was then restored and all 10 `deps::tests` pass. Note the failure mode: without
the bound the test binary *aborts*, it does not fail an assertion. That is the point —
an uncatchable crash is what the bound converts into an ordinary error.

## Why the CLI tests assert on the exit status shape

Each of the three CLI tests asserts `output.status.code().is_some()` alongside the
message. On Unix `code()` is `None` when a child is killed by a signal, so this is the
assertion that actually distinguishes "reported the depth error" from "died on SIGABRT".
Checking stderr alone would not: a stack overflow also writes to stderr.

## Edge cases covered

- Exactly at the bound (passes) and one past it (fails) — the boundary is pinned in both
  directions, so a future off-by-one is caught.
- Wide-but-shallow (2,000 tasks, 2 levels) — guards against the bound being mistakenly
  applied to task count.
- A cycle deeper than the bound still reports as a cycle: the cycle check runs first.
  Covered implicitly by the existing #513 cycle tests plus the ordering in `deps.rs`.

## Not covered

- The release-profile limit (20,000 ok / 50,000 overflow) is not exercised; the bound is
  profile-independent and sits below both, so a release-only test would add runtime
  without adding signal.
- `lanes validate` on the 1,200-chain takes ~2s, the slowest of the three. It restarts a
  walk from every task name, and `in_progress` uses a linear `position()` scan, so the
  validator is roughly quadratic in chain length. Acceptable here and out of scope, but
  worth knowing if the bound is ever raised.
