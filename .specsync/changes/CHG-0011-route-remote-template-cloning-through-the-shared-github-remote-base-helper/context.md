---
change: CHG-0011-route-remote-template-cloning-through-the-shared-github-remote-base-helper
artifact: context
---

# Context

The isolation work in CHG-0008 gave the GitHub REST base and git remote base a single
shared resolver so tests can redirect both. `src/remote.rs` was the one caller still
formatting `https://github.com/{owner}/{repo}.git` inline, so remote-template cloning
bypassed the redirection entirely.

That mattered because the mocking harness documentation claimed remote template fetch was
covered when it was not: cloning shells out to `git`, a subprocess an in-process HTTP
mock can never intercept. Either the claim or the code had to change, and the code was
the cheaper honest fix — one call site.

This lands as its own change because spec-sync refuses to widen the definition of an
already-applied change ("perform further spec changes in a new change workspace"), and
CHG-0008 had already been accepted before this call site was found.
