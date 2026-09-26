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
- [ ] `test (windows-latest)` green on this PR (ureq 3.3.0)
- [ ] `test (windows-latest)` green on the rebased #532 (ureq 3.4.2)
