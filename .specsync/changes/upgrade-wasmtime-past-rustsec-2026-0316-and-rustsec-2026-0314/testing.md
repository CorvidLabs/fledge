---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: testing
---

# Testing

## Requirement evidence

Each acceptance criterion of the requirements in `deltas/plugin-wasm.md`, and what proves it.
Test names are in `src/plugin/wasm.rs::tests`.

| Requirement | Acceptance criterion | Evidence |
|---|---|---|
| REQ-plugin-wasm-001 | reading under `/project` returns 0 | `project_scope_can_read_project_files` |
| REQ-plugin-wasm-001 | `O_CREAT` under `/project` is `EPERM`; no file on the host | `project_scope_cannot_create_files_in_project` |
| REQ-plugin-wasm-001 | write-open, with and without `O_TRUNC`, is `EPERM`; contents unchanged | `project_scope_cannot_write_or_truncate_project_files` |
| REQ-plugin-wasm-001 | `path_create_directory` under `/project` is `EPERM`; no directory | `project_scope_cannot_create_directories_in_project` |
| REQ-plugin-wasm-001 | `/project` stays read-only under the `plugin` scope | `plugin_scope_keeps_project_read_only` |
| REQ-plugin-wasm-002 | `filesystem = "plugin"` creates `<plugin_dir>/data/` | `filesystem_plugin_creates_data_dir` |
| REQ-plugin-wasm-002 | create and mkdir under `/plugin` return 0 and land in `<plugin_dir>/data/` | `plugin_scope_can_write_plugin_data_dir` |
| REQ-plugin-wasm-002 | `../escape.txt` from `/plugin` fails; nothing written beside `data/` | `plugin_scope_cannot_escape_data_dir` |
| REQ-plugin-wasm-003 | default capabilities: the module instantiates and descriptor 3 is `EBADF` | `no_filesystem_grant_has_no_preopens` (`run_fs_probe` unwraps `instantiate`, so instantiating is part of the assertion) |
| REQ-plugin-wasm-004 | TCP and UDP are switched on only inside `if capabilities.network` | Code, not a test: `build_wasi_p1` in `src/plugin/wasm.rs` (the `if capabilities.network` block). A preview 1 guest cannot observe it (in wasmtime-wasi 49.0.1, `sock_accept`, `sock_recv`, `sock_send` and `sock_shutdown` only look up an existing descriptor and return `ENOTSOCK`), and `WasiP1Ctx` does not expose the socket settings. See the open items in `context.md` |
| REQ-plugin-wasm-004 | no other socket setting, so the builder defaults hold | Code, not a test: `grep -n "allow_ip_name_lookup\|socket_addr_check" src/plugin/wasm.rs` finds nothing. In wasmtime-wasi 49.0.1, `src/ctx.rs` documents TCP, UDP and IP name lookup as "By default this is disabled", and `SocketAddrCheck::default()` in `src/sockets/mod.rs` refuses every address |
| REQ-plugin-wasm-005 | growing to exactly 256 MiB succeeds | `memory_is_capped_at_max_memory_bytes` (`run_grow(pages_at_cap - 1, false)`) |
| REQ-plugin-wasm-005 | one page past the cap returns -1 and the run does not trap | `memory_is_capped_at_max_memory_bytes` (`run_grow(pages_at_cap, true)`: the guest exits 0 only when `memory.grow` returned -1, so `Ok` also rules out a trap) |
| REQ-plugin-wasm-006 | the baseline module compiles | `engine_keeps_wasmtime_46_feature_set` |
| REQ-plugin-wasm-006 | GC, exception-handling, typed function references and wide-arithmetic modules are each refused | `engine_keeps_wasmtime_46_feature_set` |

The mutation runs under "What was verified" show the filesystem, memory and feature-set
tests fail when the sandbox they pin is weakened.

## Spec corrections

The five `## MODIFIED` sections in `deltas/plugin-wasm.md` are verbatim copies of the
living `plugin-wasm.spec.md` sections with one sentence replaced in each. What makes each
replacement true:

| Section | Replaced statement | Evidence |
|---|---|---|
| WASM Host Interface (`### Network`) | grant gives outbound TCP/UDP, no listening sockets, DNS from the host | REQ-plugin-wasm-004's evidence; `setup_linker` links all of preview 1 (`wasmtime_wasi::p1::add_to_linker_sync`), which has no call that creates a socket; wasmtime-wasi 49.0.1 documents `inherit_network()` as letting the guest bind or connect to any address |
| Security Model (guarantee 2) | no socket imports without the grant | the same `add_to_linker_sync` call links the preview 1 socket imports for every plugin; REQ-plugin-wasm-004's evidence |
| Invariants (7) | no socket imports without the grant | as above |
| Behavioral Examples (zero-capability scenario) | instantiation fails because the filesystem imports are not linked | REQ-plugin-wasm-003's evidence: the module instantiates and descriptor 3 is `EBADF` |
| Error Cases (memory limit) | the plugin traps with "plugin exceeded memory limit" | REQ-plugin-wasm-005's evidence; any other trap reaches the `_ => bail!("Plugin '{}' trapped: {}", …)` arm of `run_wasm_plugin`, and no "exceeded memory limit" string exists in `src/` |

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
| `specsync change show` after naming `plugin-wasm` (SpecSync 6.0.0) | draft, `affected_specs: [plugin-wasm]`, `no_spec_change: false`, no open questions, artifacts complete; next step is definition approval |
| SpecSync 6.0.0's approve, check and ship gates on a scratch copy of the branch, called read-only from a scratch build of the v6.0.0 tag (no approval recorded, nothing saved) | `validate_definition`, `validate_delta_files`, `validate_declared_path_ownership` and the effective-contract replay pass; `src/plugin/wasm.rs` resolves to `plugin-wasm`, and `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` to `@exact:delivery`; every REQ-plugin-wasm ID has evidence; the scoped `specsync check --spec plugin-wasm --strict` passes |
| The canonical files that materialization would write, on the scratch copy | spec diff is the five sentences, `version` 2 to 3 and one Change Log row; `specsync check --force --strict --require-coverage 100` 33 passed, 0 warnings; `fledge spec lint` 0 errors, 0 warnings |
| `specsync check --force --strict --require-coverage 100` on the branch | 33 passed, 0 warnings, file and LOC coverage 100% |
| `specsync change check --strict --require-coverage 100` (CI's form) | "Nothing to check": the change is still a draft |
| `specsync change audit` | fails only on "meaningful changed paths are not covered by an active change", because a draft covers no paths until it is approved; the no-spec draft got the same result |
| `cargo +stable test --locked --bin fledge plugin::wasm` (rustc 1.98.0) | 48 passed |
| `fledge run spec-check` | 33 specs, 0 errors, 0 warnings |

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
