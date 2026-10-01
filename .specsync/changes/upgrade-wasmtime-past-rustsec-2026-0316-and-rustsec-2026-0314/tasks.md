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
- [ ] CI green on #538: test and integration on ubuntu-latest, macos-latest,
      windows-latest; lint; audit; spec-check; intent-check
- [ ] Definition approval (pending with orc/Leif)
