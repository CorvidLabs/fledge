---
change: CHG-0011-route-remote-template-cloning-through-the-shared-github-remote-base-helper
artifact: plan
---

# Plan

1. Replace the inline URL format in `src/remote.rs::clone_repo` with
   `crate::github::remote_url(owner, repo)`.
2. Add `tests/isolation.rs` covering a clone from a local bare repository through the
   spawned binary.
3. Confirm the test genuinely exercises the seam by reverting the `remote.rs` line and
   observing it attempt a real clone.
4. Update `specs/remote/` for the new dependency on the shared helper.
5. Run the full gate.
