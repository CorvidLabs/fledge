---
change: drop-the-unused-wasmtime-compilation-cache-so-zstd-leaves-the-dependency-graph
artifact: context
---

# Context

The `audit` job (rustsec/audit-check) has failed on main since 2026-10-27 (main ac33248): three critical advisories
published 2026-10-16 hit zstd 0.13.3 and zstd-safe 7.3.0 (RUSTSEC-2026-0391, -0398, -0403; patched in zstd 0.14.3 and
zstd-safe 8.2.0). Every open fledge PR inherits the red check.

The only path to zstd is wasmtime 49's default `cache` feature (wasmtime -> wasmtime-internal-cache -> zstd 0.13).
wasmtime-internal-cache 50.0.1 still requires zstd ^0.13, so upgrading wasmtime does not help. fledge never turns the
wasmtime cache on: `create_engine` builds a plain `Config` and src/plugin/wasm.rs keeps its own `.cwasm` cache with a
wasmtime version stamp. Dropping the unused feature removes zstd entirely.

Picked up in the CorvidLabs PR pass Leif asked for in chat (t775u, 2026-10-31).
