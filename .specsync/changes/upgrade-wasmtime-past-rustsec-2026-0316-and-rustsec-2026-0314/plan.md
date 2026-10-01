---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: plan
---

# Plan

1. Branch `0xleif/fix/wasmtime-rustsec-2026-0316` from `origin/main` (`85d7c4e`).
2. Raise `wasmtime` and `wasmtime-wasi` to `49.0.1` in `Cargo.toml`, then run
   `cargo update -p wasmtime -p wasmtime-wasi`. Fall back to 48.0.3 only if 49 needs
   more than a mechanical source change.
3. Follow the wasmtime-wasi 49 API in `build_wasi_p1`: `FsPerms::ReadOnly` for
   `/project`, `FsPerms::ReadWrite` for `/plugin`, and `allow_tcp(true)` /
   `allow_udp(true)` under the network grant.
4. Diff wasmtime-wasi 46.0.3 against 49.0.1 (`ctx.rs`, `filesystem.rs`,
   `sockets/mod.rs`, CLI/clock/random defaults) and wasmtime's limits, traps and fuel
   instrumentation for anything else that changes the sandbox.
5. Add regression tests for the preopen grants, `..` confinement, the no-grant case and
   the memory cap. Run them against main's 46.0.3 as well, unchanged, and mutate the
   46.0.3 sandbox to show they fail when it is weakened.
6. Run fmt, clippy (CI's form and the `lint` task's `--all-targets`), clippy with
   `--no-default-features`, the full `cargo test --locked`, and `cargo audit` against
   the committed lock, a fresh `generate-lockfile` resolution, and main's lock as a
   negative control.
7. Record the security fix under 1.8.1 in `CHANGELOG.md`, and correct the 1.8.1 note
   that said dependencies were unchanged from 1.8.0.
8. Open the PR and watch every CI job.
