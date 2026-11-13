---
change: bump-wasmtime-to-49-0-5-to-close-dependabot-alerts-11-and-12-and-four-more-wasmtime-advisories
artifact: testing
---

# Testing

A dependency bump has no failing behavior test to write first. The evidence is the dependency graph, the advisory
report and the existing suites, run on macOS (aarch64, rustc 1.99.0, cargo 1.99.0) on origin/main add5767 plus this
change.

- `cargo tree -i wasmtime --locked`: before, wasmtime v49.0.3 with wasmtime-wasi, wasmtime-wasi-io and wiggle at
  v49.0.3; after, all four at v49.0.5. fledge is the only root.
- Family versions in Cargo.lock: before, 13 cranelift-* crates at 0.136.3 and 19 wasmtime / wasmtime-* / pulley-* /
  wiggle* crates at 49.0.3; after, 0.136.5 and 49.0.5. `git diff --numstat Cargo.lock` is 64/64: one version and
  one checksum line for each of the 32 packages, nothing else. Package count stays 353.
- `cargo audit` (cargo-audit 0.22.2, advisory DB fetched 2026-11-13, 1509 advisories): main's lock reports 8
  vulnerabilities, including RUSTSEC-2026-0378, -0379, -0420, -0422, -0423 and -0424 on wasmtime / wasmtime-wasi
  49.0.3; this lock reports 2, rustls-webpki RUSTSEC-2026-0382 and webpki-roots RUSTSEC-2026-0381 (not in scope).
  Exit is 1 both times because of those two.
- `cargo deny` is installed but the repo has no deny.toml, so it was not run.
- `fledge run test -- --locked wasm`: 58 passed, 0 failed (48 in plugin::wasm::tests, including engine creation,
  .cwasm compile/cache/version-stamp, sandbox capability denial, fuel and memory limits; plus 10 wasm tests in
  plugin::tests, plugin::install and plugin::update).
- `fledge run fmt` (cargo fmt --check): clean.
- `cargo clippy --locked --all-targets -- -D warnings`: clean.
- `fledge run test -- --locked` (full suite, run under the shared heavy-tests lock): 13 test binaries, 1319 passed,
  0 failed, 0 ignored.
