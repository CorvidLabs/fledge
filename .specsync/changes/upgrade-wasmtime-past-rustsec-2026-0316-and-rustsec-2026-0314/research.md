---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: research
---

# Research

## Which release line

| Line | Patched? | MSRV | Notes |
|---|---|---|---|
| 49.0.1 | yes | 1.96 | newest; chosen |
| 48.0.3 | yes | 1.95 | fallback if 49 had needed more source changes |
| 47.x | no | 1.94 | also inside RUSTSEC-2026-0315's affected range (`unaffected < 47.0.0`) |
| 36.0.16 (LTS) | yes | | ten majors back from 46; rejected |

49.0.1 built after a four-line change in `build_wasi_p1`, so 48.0.3 was never built.
crates.io's `rust_version` field was read for each line.

## Lockfile movement

`cargo update -p wasmtime -p wasmtime-wasi` after the requirement bump locked 47
packages, all inside the wasmtime tree:

- wasmtime, wasmtime-wasi, wasmtime-wasi-io, wasmtime-environ, every
  `wasmtime-internal-*`, wiggle, wiggle-generate, wiggle-macro, pulley-interpreter,
  pulley-macros: 46.0.3 to 49.0.1
- cranelift-*: 0.133.3 to 0.136.1
- wasm-encoder, wasmparser, wasmprinter, wasm-compose, wit-parser: 0.251.0 to 0.258.0
- cap-primitives 3.4.6 to 4.0.3, io-extras 0.18.4 to 0.19.0, object 0.39.1 to 0.40.0,
  cpp_demangle 0.4.5 to 0.5.1
- toml 0.9.12 to 1.1.6, used only by `wasmtime-internal-cache`. fledge's own
  `toml = "0.8"` stays 0.8.23.
- removed: cap-std, cap-fs-ext, cap-net-ext, cap-time-ext, unicode-xid
- added: io-lifetimes 3.0.1, wasm-metadata 0.258.0, wit-component 0.258.0

Nothing outside the wasmtime family moved in the lockfile. Feature unification did
change: `cap-primitives` 3.4.6 had been enabling `Win32_Security` on `windows-sys`
0.59, which fledge's Windows job-object code needs without declaring it (see context).

## Source read to confirm equivalence

- `wasmtime-wasi` `ctx.rs`, 46.0.3 against 49.0.1: the only differences are
  `preopened_dir`'s signature and the socket-default doc comments.
- `filesystem.rs`: every 46.0.3 `DirPerms::MUTATE` / `FilePerms::WRITE` check has a
  49.0.1 `write_not_permitted()` check at the same site. 46.0.3's `DirPerms::READ`
  checks have no 49.0.1 counterpart, because both `FsPerms` variants allow reads, and
  both fledge preopens always granted `READ`. `open_at` refuses create or write-open
  under a read-only preopen in both versions. `open_mode` is `READ` for `/project`
  and `READ | WRITE` for `/plugin` in both.
- `sockets/mod.rs`: `AllowedNetworkUses::default()` was `{ip_name_lookup: false, udp:
  true, tcp: true}` in 46.0.3 and is all `false` in 49.0.1. `inherit_network()` is
  unchanged: it only installs an allow-all `socket_addr_check`.
- `cli.rs`, `clocks.rs`, `random.rs` defaults: unchanged.
- `wasmtime` `runtime/limits.rs`: byte-identical. `Trap::OutOfFuel` and
  `Trap::Interrupt` still exist and still downcast from the call error.
- `wasmtime-internal-cranelift` fuel instrumentation: `Loop`, `If`, branches and `End`
  are still fuel points; `CallRef` now reloads fuel; bulk memory ops are charged by
  size.

## Advisory text

- GHSA-jqpg-j7w6-42pr: "This vulnerability only affects the `Val` API of hosts. Hosts
  that use `bindgen!` and otherwise statically-typed APIs are unaffected."
- GHSA-j2g9-4prp-pf6h: wasip1 `path_filestat_set_times` / `fd_filestat_set_times`,
  wasip2/wasip3 `set-times` / `set-times-at`. The fix in 49.0.1 is in
  `p2/host/filesystem.rs::systemtime_from`, which preview 1 reaches only with
  nanoseconds already reduced below one second.
