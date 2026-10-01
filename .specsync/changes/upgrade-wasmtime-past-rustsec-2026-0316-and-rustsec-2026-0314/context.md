---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: context
---

# Context

## What led here

CI on `main` went red on exactly one job, `audit`, at `85d7c4e` (the v1.8.1 release
commit, which is neither tagged nor published; crates.io still has 1.8.0). Run
36787734566, `rustsec/audit-check@v2.0.0`, advisory DB `9b3a3b73` (2026-09-30):

| Advisory | Crate | Title |
|---|---|---|
| RUSTSEC-2026-0316 (GHSA-jqpg-j7w6-42pr) | wasmtime 46.0.3 | Dynamic record lifting can allocate beyond the hostcall fuel limit |
| RUSTSEC-2026-0314 (GHSA-j2g9-4prp-pf6h) | wasmtime-wasi 46.0.3 | Guest can panic host through filesystem datetime overflow |

Both were published 2026-09-24. Patched: `>=36.0.16,<37`, `>=48.0.3,<49`, `>=49.0.1`.
The two `unmaintained` warnings in the same run (`instant`, `number_prefix`) are
informational and do not fail the job.

The audit job runs `cargo generate-lockfile` before auditing, so it checks a fresh
resolution of `Cargo.toml`, not the committed `Cargo.lock`. The `46.0.1` requirement
resolves to 46.0.3 there whatever the lock says. The fix has to move the requirement
itself, and the lock with it so `--locked` builds agree.

The 1.8.1 tag waits on this change.

## Exposure

fledge loads plugins as core modules (`Module::new`) and links only WASI preview 1
(`wasmtime_wasi::p1::add_to_linker_sync`). On that footing neither advisory looks
reachable:

- **RUSTSEC-2026-0316** is in the component-model `Val` API ("This vulnerability only
  affects the `Val` API of hosts"). fledge never instantiates a component.
- **RUSTSEC-2026-0314** is `Duration::new` panicking when nanoseconds overflow into a
  seconds field already at `u64::MAX`. Preview 1 passes a timestamp as one `u64` of
  nanoseconds and `systimespec` splits it into `ts / 1e9` seconds plus `ts % 1e9`
  nanoseconds, so the nanoseconds are always below one second and the overflow cannot
  form. The GHSA lists wasip1 as affected; a preview 1 guest calling
  `path_filestat_set_times` with `u64::MAX`, `i64::MAX`, `i64::MIN` and
  `i64::MIN + 999_999_999` against 46.0.3 returned normally in every case (errno 0 on
  `/plugin`, 63 on the read-only `/project`). Checked on macOS only.

The upgrade ships anyway. The plugin runtime is fledge's security boundary, `cargo
audit` is a required check, and "not reachable today" depends on fledge staying on
core modules and preview 1.

## What changed between 46.0.3 and 49.0.1 that fledge touches

| Area | 46.0.3 | 49.0.1 | fledge change |
|---|---|---|---|
| Preopen permissions | `preopened_dir(host, guest, DirPerms, FilePerms)` | `preopened_dir(host, guest, FsPerms)`, `ReadOnly` or `ReadWrite` | `/project`: `READ` + `READ` becomes `ReadOnly`; `/plugin`: `all()` + `all()` becomes `ReadWrite` |
| Socket defaults | TCP on, UDP on, IP name lookup off | all three off | a `network` grant now calls `allow_tcp(true)` and `allow_udp(true)` after `inherit_network()` |
| Filesystem backend | `cap-std` | cap-primitives' sandboxed resolver vendored as `filesystem::primitives` | none; `..` confinement is now covered by a test |
| Fuel instrumentation | | `call_ref` reloads fuel (the RUSTSEC-2026-0315 fix; 46 was unaffected), bulk memory ops are charged by size | none; same `FUEL_LIMIT` |
| WebAssembly proposals on by default | WASM 2.0 plus multi-memory, relaxed SIMD, tail calls, extended const, memory64 and threads | adds GC, exception handling and typed function references (since 47) and wide arithmetic (since 49) | `create_engine` turns those four off, back to 46.0.3's set |
| `StoreLimits`, `Trap::OutOfFuel`, `Trap::Interrupt`, epochs | | unchanged (`limits.rs` is byte-identical) | none |
| CLI context defaults (env, args, stdin, stderr), clocks, random | | unchanged | none |
| `windows-sys` 0.59 features | `cap-primitives` 3.4.6 (under `cap-std`) enabled `Win32_Security` | `cap-primitives` 4.0.3 no longer enables it on 0.59 | fledge declares `Win32_Security` itself |

## The Windows build break the upgrade exposed

The first CI run on #538 failed `test (windows-latest)` at compile time, in fledge's
own code, not wasmtime's:

```text
error[E0432]: unresolved import `windows_sys::Win32::System::JobObjects::CreateJobObjectW`
   --> src\lanes\execute.rs:659:76
```

In windows-sys 0.59, `CreateJobObjectW` is behind `#[cfg(feature = "Win32_Security")]`,
because its first parameter is a `SECURITY_ATTRIBUTES` pointer. fledge declares only
`Win32_Foundation` and `Win32_System_JobObjects`. On main the missing feature arrived
through feature unification: `cap-primitives` 3.4.6, pulled in by wasmtime-wasi 46's
`cap-std`, enables `Win32_Security` on windows-sys 0.59. wasmtime-wasi 49 dropped
`cap-std`, so nothing else turns the feature on. `cargo tree --target
x86_64-pc-windows-msvc -e features -i windows-sys@0.59.0` shows the change: on main the
only `Win32_Security` edge comes from `cap-primitives`; with `--no-default-features`,
main has none at all, so a Windows build without the `wasm` feature was already broken
there. fledge now declares `Win32_Security` itself. That is a feature flag only: no new
crate, no `Cargo.lock` change.

## Constraints

- The crate version stays 1.8.1. No tag, no publish, no crates.io.
- CI runners are not touched.
- wasmtime 49 needs rustc 1.96 (48 needs 1.95; 46 needed 1.94). CI builds on `stable`
  (1.98.0 today), so CI is unaffected. People building from source need 1.96 or newer.

## Out of scope (noticed, left alone)

- `Cargo.toml` declares `rust-version = "1.89"`, which was already below the 1.94 that
  wasmtime 46 needed. Raising it is a separate decision: clippy reads `rust-version`
  for its MSRV-aware lints, so changing it could surface new `-D warnings` failures
  unrelated to this fix.
- `plugin-wasm.spec.md` says a network grant gives outbound TCP/UDP with DNS from the
  host. In preview 1 a guest has no call that creates a socket, and IP name lookup is
  off, so a network grant reaches nothing usable today. That was true on 46.0.3 too;
  this change keeps the grant's configuration identical rather than fixing the gap.
- The spec's Error Cases say a plugin over the memory cap traps with "plugin exceeded
  memory limit". In fact `memory.grow` returns -1 and the plugin carries on, on both
  46.0.3 and 49.0.1. The new test pins the real behaviour; the spec text is untouched.
- `exit_code_42_returns_error_with_code` passes because the plugin is named
  `test-exit42`. `run_wasm_plugin` formats the wasmtime error with `{}`, which drops
  the cause chain, so the exit code never reaches the message.
- `StoreLimits` caps each memory at 256 MiB but sets no `table_elements` limit, so a
  guest can `table.grow` a `funcref` table far past that: 50 million elements (about
  400 MB of host memory) succeeded on 46.0.3 and 49.0.1 alike. Multi-memory and shared
  memories are accepted on both too. Fuel and the wall clock still bound the run. That
  predates this change and is left for a separate fix.
- The committed `Cargo.lock` carries `yoke-derive 0.8.3`, which is yanked
  (`cargo audit` warns). It predates this change, and CI's fresh resolution does not
  pick it.
