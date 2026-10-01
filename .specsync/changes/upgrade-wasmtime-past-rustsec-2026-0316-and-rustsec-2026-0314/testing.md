---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: testing
---

# Testing

## Automated

New tests in `src/plugin/wasm.rs` (run by CI's `cargo test --verbose --locked` on all
three OSes):

| Test | Asserts |
|---|---|
| `project_scope_can_read_project_files` | `path_open` for read under `/project` returns 0 |
| `project_scope_cannot_create_files_in_project` | create under `/project` returns EPERM (63); no file on the host |
| `project_scope_cannot_write_or_truncate_project_files` | write-open, and write-open with `O_TRUNC`, return EPERM; contents unchanged |
| `project_scope_cannot_create_directories_in_project` | `path_create_directory` under `/project` returns EPERM; no directory |
| `plugin_scope_can_write_plugin_data_dir` | create and mkdir under `/plugin` return 0 and land in `<plugin_dir>/data/` |
| `plugin_scope_keeps_project_read_only` | `/project` is still read-only when the scope is `plugin` |
| `plugin_scope_cannot_escape_data_dir` | `../escape.txt` from `/plugin` fails; nothing written beside `data/` |
| `no_filesystem_grant_has_no_preopens` | without a filesystem grant fd 3 is EBADF |
| `memory_is_capped_at_max_memory_bytes` | `memory.grow` to exactly 256 MiB succeeds; one page more returns -1 |
| `engine_keeps_wasmtime_46_feature_set` | a baseline module (bulk memory, sign extension, saturating truncation, multi-value, reference types, tail calls) compiles; a GC, an exception-handling, a typed function references and a wide-arithmetic module are each refused |

Existing tests still cover fuel exhaustion, capability denial at instantiation, the
`.cwasm` cache and its version stamp (`wasmtime_version_derived_from_cargo_toml`,
`cache_invalid_on_version_mismatch`).

## What was verified

All local runs on macOS (aarch64), rustc/clippy 1.98.0 (`stable`, the toolchain CI
uses).

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --locked -- -D warnings` (CI's lint form) | clean |
| `fledge run lint` (`cargo clippy --all-targets -- -D warnings`) | clean |
| `cargo clippy --locked --no-default-features -- -D warnings` | clean |
| `cargo test --verbose --locked` | all pass: 1096 unit tests (1087 on main plus the 9 new ones, 47 of them in `plugin::wasm`) and every integration test binary |
| New tests, unchanged, on main's 46.0.3 code | same results, same errno values |
| Same tests on 46.0.3 with `/project` widened to `all()` / `all()` and the memory limiter removed | 5 fail: the four read-only tests and the memory cap |
| Fuel exhaustion (infinite loop through `run_wasm_plugin`) | "exceeded its compute budget" after about 6.7 s, on 49.0.1 and 46.0.3 alike |
| Epoch deadline (deadline 1, epoch bumped after 300 ms) | `Trap::Interrupt` after about 306 ms, on 49.0.1 and 46.0.3 alike |
| `path_filestat_set_times` with extreme timestamps, preview 1 | no panic on either version; 0 on `/plugin`, 63 on `/project` |
| `cargo audit` on the branch `Cargo.lock`, advisory DB `9b3a3b7` (the commit CI used) | exit 0, 0 vulnerabilities |
| `cargo audit` on a fresh `cargo generate-lockfile` resolution, same DB | exit 0, 0 vulnerabilities; wasmtime, wasmtime-wasi and wiggle at 49.0.1 |
| `cargo audit` on main's `Cargo.lock`, same DB (negative control) | exit 1: RUSTSEC-2026-0316 (wasmtime 46.0.3), RUSTSEC-2026-0314 (wasmtime-wasi 46.0.3) |
| `cargo deny` | not run; the repository has no `deny.toml` |
| `test (windows-latest)` on #538's first run (36804407887) | failed to compile: `CreateJobObjectW` missing without `Win32_Security` |
| `cargo tree --target x86_64-pc-windows-msvc -e features -i windows-sys@0.59.0` | main: `Win32_Security` only via `cap-primitives` 3.4.6, none with `--no-default-features`; branch: enabled by fledge, with and without default features |
| Windows compile, locally | not run; no Windows target installed here, so CI's `windows-latest` cells are the check |
| Review: probe modules for each proposal on main's 46.0.3 and on 49.0.1 before the feature-set commit | 46.0.3 refuses GC, exceptions, typed function references and wide arithmetic; 49.0.1 accepted all four |
| `engine_keeps_wasmtime_46_feature_set`, unchanged, on main's 46.0.3 code | passes |
| Same test on 49.0.1 without the four `wasm_*(false)` calls | fails (GC accepted) |
| `cargo test --locked` after the feature-set commit (macOS, 1.98.0) | all pass: 1097 unit tests and every integration test binary |
| fmt and clippy (CI form, `--all-targets`, `--no-default-features`) after the feature-set commit | clean |

## Acceptance signals

- The `audit` job is green on the PR.
- `test` and `integration` are green on ubuntu-latest, macos-latest and windows-latest,
  with the new sandbox tests among them.

## Rejection signals

- Any of the four `/project` read-only tests passing with `/project` writable. The
  mutation run shows they fail.
- `memory_is_capped_at_max_memory_bytes` passing without the limiter. The mutation run
  shows it fails.
- `engine_keeps_wasmtime_46_feature_set` passing with wasmtime's defaults left on. The
  mutation run shows it fails.
- `cargo audit` on a fresh resolution still finding either advisory, which would mean
  the requirement did not move.
