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
- [x] `test (windows-latest)` green on #535 (ureq 3.3.0), in CI run 36257383427
- [x] `test (windows-latest)` green with ureq 3.4.2, in CI run 36257571327: a
      `workflow_dispatch` probe of `main` + this fix + #532's release commit rebased
      onto it
- [x] `test (windows-latest)` green on the release PR itself, in CI run 36260828952. That
      PR is #536 (v1.8.1), which replaced the closed #532 because 1.8.0 was already on
      crates.io; it carries ureq 3.4.2 on top of this change
