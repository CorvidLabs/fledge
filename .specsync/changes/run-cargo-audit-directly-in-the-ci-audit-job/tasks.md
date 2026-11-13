---
change: run-cargo-audit-directly-in-the-ci-audit-job
artifact: tasks
---

# Tasks

- [x] Replace the rustsec/audit-check step with `cargo install cargo-audit --locked` and `cargo audit`.
- [x] Check locally that `cargo audit` exits 0 with today's two unmaintained warnings.
- [ ] Leif reviews and approves the change.
- [ ] CI's `audit` job passes on this PR.
