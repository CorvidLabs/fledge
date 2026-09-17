---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
artifact: design
---

# Design

No user-facing layout or UI surface; the design decision here is where the check sits
in the walk and what it is expressed as.

## Placement

The guard goes **after** the `completed` short-circuit and the cycle check, and
**before** `in_progress.push`:

```text
completed.contains(name)      → Ok(())        // already walked
in_progress.position(name)    → cycle error   // genuine back edge
in_progress.len() >= MAX      → depth error   // new
in_progress.push(name)                        // descend
```

That ordering matters. A cycle reached at depth 1,001 should still be reported as a
cycle, not as a depth overflow — the cycle is the more specific and more actionable
diagnosis. Placing the bound after the cycle check preserves that, and keeps every
existing error message reachable.

Because the check precedes `push`, `in_progress.len()` is exactly the number of
ancestors, so a chain of `MAX_TASK_DEPTH` nodes is the last one that fits.

## A constant, not configuration

`MAX_TASK_DEPTH` is a `pub(crate) const`, not a `fledge.toml` knob. A configurable
depth would invite raising it to work around a malformed graph, which is the situation
the error is meant to surface. Its value is tied to a property of the build (the debug
stack limit), not to anything a project should tune.

## Error text

```text
Dependency chain deeper than 1000 tasks (reached 't1000') — check task deps for an unintended chain
```

Names the bound, names where it stopped, and says what to look at. It follows the
existing `Circular dependency detected: a → b → a` house style: a statement of what was
found, then the evidence.
