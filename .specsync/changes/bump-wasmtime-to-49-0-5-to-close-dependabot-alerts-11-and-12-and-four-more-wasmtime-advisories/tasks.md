---
change: bump-wasmtime-to-49-0-5-to-close-dependabot-alerts-11-and-12-and-four-more-wasmtime-advisories
artifact: tasks
---

# Tasks

- [x] Pin the wasmtime family in Cargo.lock: `cargo update -p wasmtime-wasi --precise 49.0.5` (wasmtime, every
      wasmtime-internal-*, pulley-* and wiggle* crate to 49.0.5; every cranelift-* crate to 0.136.5).
- [x] Raise the wasmtime and wasmtime-wasi requirements in Cargo.toml from 49.0.2 to 49.0.5.
- [x] Restore the seven non-family Cargo.lock blocks whose `windows-sys` edge cargo reassigned, and confirm
      `cargo metadata --locked` / `cargo tree --locked --target all` leave the lock unchanged.
- [x] Record `cargo tree -i wasmtime`, the family versions in Cargo.lock and `cargo audit` before and after.
- [x] Run the wasm plugin tests and `cargo fmt --check`.
- [x] Run `cargo clippy --locked --all-targets -- -D warnings`.
- [x] Run the full `cargo test --locked` suite.
