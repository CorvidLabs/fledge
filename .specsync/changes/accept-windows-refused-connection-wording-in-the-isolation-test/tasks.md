---
change: accept-windows-refused-connection-wording-in-the-isolation-test
artifact: tasks
---

# Tasks

- [x] Read the failing `test (windows-latest)` log on #532 and record the exact stderr
- [x] Confirm `main`'s Windows cell is green, and find why: ureq 3.3.0 on `main` vs 3.4.2
      in the release lockfile
- [x] Confirm the cause in ureq's source: 3.3.0 synthesizes "Connection refused", 3.4
      returns the OS error
- [x] Accept `actively refused` / `os error 10061` alongside `connection refused`, keeping
      the loopback-base assertion and excluding a bare `failed`
- [x] Run the isolation tests on both lockfiles locally (macOS)
- [x] `test (windows-latest)` green on this PR (ureq 3.3.0), in CI run 36257383427
- [x] `test (windows-latest)` green with ureq 3.4.2, in CI run 36257571327: a
      `workflow_dispatch` probe of `main` + this fix + #532's release commit rebased
      onto it
- [ ] `test (windows-latest)` green on #532 itself, once it is rebased onto this change
