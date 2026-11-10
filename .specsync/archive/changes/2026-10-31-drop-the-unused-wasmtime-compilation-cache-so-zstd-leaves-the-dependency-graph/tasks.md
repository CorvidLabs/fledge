---
change: drop-the-unused-wasmtime-compilation-cache-so-zstd-leaves-the-dependency-graph
artifact: tasks
---

# Tasks

- [x] Set `default-features = false` on wasmtime and list every wasmtime 49.0.3 default feature except `cache`.
- [x] `cargo update -w`: Cargo.lock only removes packages (zstd, zstd-safe, zstd-sys, wasmtime-internal-cache and
      their now-unused dependencies).
- [x] Run tests, clippy and fmt on Linux.
