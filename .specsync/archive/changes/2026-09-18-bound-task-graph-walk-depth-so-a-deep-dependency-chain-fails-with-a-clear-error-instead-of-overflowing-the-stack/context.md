---
change: bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack
artifact: context
---

# Context

## What led here

PR #513 fixed a real bug — a diamond dependency (`a → [b, c]`, `b → d`, `c → d`)
was reported as a cycle — by replacing three separate DFS implementations with one
shared walker, `deps::walk_task_graph`. That fix is correct and stays.

What it also did was change *how* the graph is descended. The previous
`src/lanes/validate.rs` used an explicit heap stack
(`let mut stack = vec![..]; while let Some(x) = stack.pop()`), which could not
overflow no matter how deep the graph. The shared walker recurses instead, once per
level, with no bound. On a long chain that exhausts the thread stack:

```
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting     (exit 134)
```

Measured on f659d69, a chain of N tasks each depending on the next:
release build 20,000 ok / 50,000 overflow; debug build 16,000 ok / 20,000 overflow.

A SIGABRT is not a failure a caller can catch, report, or test around — it takes the
process with it. That is the whole of what this change fixes.

#513's test plan checks off "20,000-deep dependency chain validates without stack
overflow", but no such test exists in the tree, and at 20,000 in a debug build (which
is what CI builds) it would have aborted the test binary rather than passed.

## Decisions

**A depth bound, not an iterative rewrite.** Both fix the abort. The bound was chosen
because it is the smaller change: an iterative post-order walk has to carry its own
enter/exit marks to keep `on_ready` firing after a node's dependencies, and the exact
ordered cycle-path reporting (`a → b → c → a`, not a `HashSet` iteration) is pinned by
four tests from #513. Restructuring the traversal to remove a limit that no real task
graph approaches would risk those semantics for no practical gain. A chain deeper than
1,000 is a generated or malformed `fledge.toml`, and a named error serves that case
better than silently succeeding.

**1,000 levels.** Far enough under the debug overflow point (~16,000) that the guard,
not the stack, is what a deep graph meets — which is what lets the regression test run
in CI at all.

**The bound is on nesting, not task count.** `in_progress` is the recursion stack, so
its length is the current depth. A wide, shallow graph — one root with thousands of
leaf deps — is unaffected, and `depth_is_nesting_not_total_nodes` pins that.

## Ruled out

- Raising the thread stack size, or spawning the walk on a big-stack thread: moves the
  cliff instead of removing it, and the abort is still uncatchable when reached.
- Leaving `lanes validate` iterative and bounding only the other two callers: the three
  call sites were deliberately unified by #513, and splitting them again is the drift
  that fix existed to prevent.

## Dead ends worth knowing

**Do not pre-write a new requirement into `specs/<module>/requirements.md` when the
change also declares it in `deltas/<module>.md` as `## ADDED`.** `change approve` rejects
it with `cannot add existing block <REQ-ID> with different content; use ## MODIFIED for
requirements already present in the living tree` — and the block text can be
*byte-identical*, so the message reads like a formatting problem when it is not. The
delta is the single source for a new requirement; `change check` materializes it into the
living tree on approval. Relabelling to `## MODIFIED` clears the error but misdescribes
an addition as an edit in the archived evidence, which is worse than the error.

**A change declaring `affected_specs` needs a `deltas/<module>.md` per module**, or
`change approve` fails with `semantic delta modules must exactly match affected specs`.
The interview does not scaffold these, so they are easy to miss until approval.
