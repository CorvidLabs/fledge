---
change: keep-config-init-and-config-command-tests-off-the-operator-config
artifact: tasks
---

# Tasks

- [x] Reproduce `config init` with a disposable `HOME` and sentinel, including the no-override write, the bad destination, the existing file, and a network-denied run
- [x] Add child-scoped helpers on `TempEnv` (`apply_env`, network confinement, home sentinel planting)
- [x] Cover `config init` in `tests/config.rs` and point the `config set` / `config add` tests in `tests/main.rs` at the same helper
- [x] Record the boundary in the config testing, context, and tasks companions
- [ ] Human definition approval (`specsync change approve`). Not granted in this session

- [x] Reject ParentDir traversal before a Unix sentinel write, with a discriminating old-guard failure and new-guard passing regression.
- [x] Verify config remove actually removes the value from both CLI output and persisted override TOML.
- [x] Bound sentinel claims to Unix and record current config spec version13 plus the root-owned SpecSync6.1 workflow path.
