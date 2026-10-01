---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: tasks
---

# Tasks

- [x] Read the failing `audit` job on `main` (run 36787734566) and record the two
      advisories and their patched ranges
- [x] Confirm the audit job checks a fresh `generate-lockfile` resolution, so the
      `Cargo.toml` requirement has to move
- [x] Raise `wasmtime` and `wasmtime-wasi` to `49.0.1`; targeted
      `cargo update -p wasmtime -p wasmtime-wasi`
- [x] Port `build_wasi_p1` to `FsPerms`; re-enable TCP and UDP under the network grant
- [x] Diff wasmtime-wasi and wasmtime 46.0.3 against 49.0.1 for other sandbox changes
- [x] Add tests for the `/project` and `/plugin` grants, `..` confinement, no
      preopens without a grant, and the memory cap
- [x] Run the new tests unchanged against main's 46.0.3, and against a deliberately
      weakened 46.0.3 sandbox
- [x] fmt, clippy (CI form, `--all-targets`, `--no-default-features`),
      `cargo test --locked` and `cargo audit` locally (macOS, rustc 1.98.0)
- [x] CHANGELOG entry under 1.8.1
- [x] Declare `windows-sys` `Win32_Security` after `test (windows-latest)` failed to
      compile `CreateJobObjectW` on #538's first run (run 36804407887)
- [x] Turn off the proposals wasmtime 47 and 49 enabled by default (GC, exceptions,
      typed function references, wide arithmetic) and pin the feature set with a test
- [x] CI green on #538: test and integration on ubuntu-latest, macos-latest,
      windows-latest; lint; audit; spec-check; intent-check (run 36809320239 on 8964ca0)
- [x] Declare `plugin-wasm`, the owner of `src/plugin/wasm.rs`, after `change ship`
      refused the no-spec draft; write its delta (REQ-plugin-wasm-001 to 006 and the
      five spec sentences they contradict) and map each requirement to its evidence

Lifecycle (not implementation tasks): definition approval by Leif, `change check`,
review and ship before merge; tag and publish 1.8.1 after merge.
