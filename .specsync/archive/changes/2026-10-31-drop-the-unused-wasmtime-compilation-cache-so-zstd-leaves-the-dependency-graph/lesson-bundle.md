# Lesson bundle — drop-the-unused-wasmtime-compilation-cache-so-zstd-leaves-the-dependency-graph

Material for folding this change's lessons into the affected specs' `context.md`.
Synthesise from what actually happened below; do not restate the change description.

## What this change was

- **Title**: Drop the unused wasmtime compilation cache so zstd leaves the dependency graph
- **Kind**: BugFix
- **Paths**: Cargo.toml, Cargo.lock
- **Acceptance**: cargo audit no longer reports RUSTSEC-2026-0391, RUSTSEC-2026-0398 or RUSTSEC-2026-0403 because zstd, zstd-safe and zstd-sys are absent from Cargo.lock (cargo tree -i zstd finds no package); the only manifest change is wasmtime's feature list, which keeps every wasmtime 49 default feature except cache; Cargo.lock only removes packages; cargo test --locked, cargo clippy --locked --all-targets -D warnings and cargo fmt --check pass; CI's audit job passes on this PR.

## Evidence

- Verification commit: `0f06f522a18f45cba1d679134d8ed0ccd0cdeebb`
- Base commit: `ac33248cae6e4b4ef16b82d455d814d755b2bd3a`
- Verified by: `specsync check (no spec in scope)`

## From the change's context.md

# Context

The `audit` job (rustsec/audit-check) has failed on main since 2026-10-27 (main ac33248): three critical advisories
published 2026-10-16 hit zstd 0.13.3 and zstd-safe 7.3.0 (RUSTSEC-2026-0391, -0398, -0403; patched in zstd 0.14.3 and
zstd-safe 8.2.0). Every open fledge PR inherits the red check.

The only path to zstd is wasmtime 49's default `cache` feature (wasmtime -> wasmtime-internal-cache -> zstd 0.13).
wasmtime-internal-cache 50.0.1 still requires zstd ^0.13, so upgrading wasmtime does not help. fledge never turns the
wasmtime cache on: `create_engine` builds a plain `Config` and src/plugin/wasm.rs keeps its own `.cwasm` cache with a
wasmtime version stamp. Dropping the unused feature removes zstd entirely.

Picked up in the CorvidLabs PR pass Leif asked for in chat (t775u, 2026-10-31).

## From the change's testing.md

# Testing

Linux, 2026-10-31, on main ac33248 plus this change:

- `cargo tree -i zstd`: no such package in the graph.
- `cargo test --locked`: all suites pass (1097 unit tests plus every integration suite, 0 failed).
- `cargo clippy --locked --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `cargo audit` could not run locally (the box could not fetch the advisory database); CI's audit job is the check.

## Where these lessons go

This change declared no affected specs, so there is no module context to fold into.
