---
change: keep-config-init-and-config-command-tests-off-the-operator-config
artifact: tasks
---

# Tasks

- [x] Reproduce `config init` with a disposable `HOME` and sentinel, including the no-override write, the bad destination, the existing file, and a network-denied run
- [x] Add child-scoped helpers on `TempEnv` (`apply_env`, network confinement, home sentinel planting)
- [x] Cover `config init` in `tests/config.rs` and point the `config set` / `config add` tests in `tests/main.rs` at the same helper
- [x] Record the boundary in the config testing, context, and tasks companions
- [x] Definition approval recorded natively (`change approve --actor 0xLeif`) under Leif's standing 2026-11-10 authorization for spec-conforming bug fixes, relayed by the coordinator

- [x] Reject ParentDir traversal before a Unix sentinel write, with a discriminating old-guard failure and new-guard passing regression.
- [x] Verify config remove actually removes the value from both CLI output and persisted override TOML.
- [x] Bound sentinel claims to Unix and record current config spec version13 plus the root-owned SpecSync6.1 workflow path.

- [x] Declare all eight delivered test, direct-companion and workflow paths in the native draft record, including `.github/workflows/ci.yml`.
- [x] Document the existing CI binary and Trust SpecSync selectors 6.1.0, preserving both workflow bytes, the Trust 1.2.1 pin and existing gate policies; declare both workflows as delivery metadata.
- [x] Coordinator ran the actual trust gate on this worktree (`fledge trust verify`: lifecycle, contract, risk and provenance stages ran; trust gate passed). Hosted CI on the exact final head remains a merge gate, not a task.
- [x] Scoped implementation review and finalization are recorded with `change review`/`change finalize` under the same standing authorization; they are lifecycle states tracked natively, not open work items.
