---
change: CHG-0008-fix-diamond-task-dependencies-falsely-rejected-as-circular
artifact: design
---

# Design

## Approach

One shared helper in `src/deps.rs` used by all three buggy call sites:

- `in_progress`: the recursion stack. Insert before walking deps, remove after
  returning. A hit is a genuine cycle.
- `completed`: nodes whose subgraph has already been walked. A hit returns `Ok`
  immediately so a shared dep is not re-run and is not treated as a back edge.

Cycle errors report the ordered walk from the back-edge target around the stack
(`a → b → a`), not a `HashSet` iteration.

`validate.rs` calls the same recursive helper instead of its pop-stack walk.
`check_dep_cycle` in `src/lanes/mod.rs` is left as-is: it already backtracks and
is the dry-run path.

## Alternatives considered

Deleting the three detectors and calling only `check_dep_cycle` would fix
detection but not run-once dedup during execution. Executors still need a
separate `completed` set. Putting both sets in one helper keeps the three
sites from drifting again.
