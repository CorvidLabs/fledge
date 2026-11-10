---
change: drop-the-unused-wasmtime-compilation-cache-so-zstd-leaves-the-dependency-graph
artifact: testing
---

# Testing

Linux, 2026-10-31, on main ac33248 plus this change:

- `cargo tree -i zstd`: no such package in the graph.
- `cargo test --locked`: all suites pass (1097 unit tests plus every integration suite, 0 failed).
- `cargo clippy --locked --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `cargo audit` could not run locally (the box could not fetch the advisory database); CI's audit job is the check.
