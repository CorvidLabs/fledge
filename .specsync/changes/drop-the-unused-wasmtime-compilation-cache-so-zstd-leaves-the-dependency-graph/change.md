---
id: drop-the-unused-wasmtime-compilation-cache-so-zstd-leaves-the-dependency-graph
state: approved
type: bug_fix
base_commit: ac33248cae6e4b4ef16b82d455d814d755b2bd3a
---

# Drop the unused wasmtime compilation cache so zstd leaves the dependency graph

## Intent

Drop the unused wasmtime compilation cache so zstd leaves the dependency graph

## Affected Canonical Specs

- None

## Acceptance Criteria

- cargo audit no longer reports RUSTSEC-2026-0391, RUSTSEC-2026-0398 or RUSTSEC-2026-0403 because zstd, zstd-safe and zstd-sys are absent from Cargo.lock (cargo tree -i zstd finds no package); the only manifest change is wasmtime's feature list, which keeps every wasmtime 49 default feature except cache; Cargo.lock only removes packages; cargo test --locked, cargo clippy --locked --all-targets -D warnings and cargo fmt --check pass; CI's audit job passes on this PR.

## No-spec Rationale

Dependency feature selection only. fledge never enables wasmtime's built-in compilation cache (it keeps its own .cwasm cache in src/plugin/wasm.rs), so no specified plugin behaviour, CLI surface or file format changes.
