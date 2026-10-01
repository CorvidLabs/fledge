---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
artifact: design
---

# Design

## Move the requirement, not only the lock

`wasmtime` and `wasmtime-wasi` go from `"46.0.1"` to `"49.0.1"` in `Cargo.toml`. CI's
audit job resolves from scratch (`cargo generate-lockfile`), and so does
`cargo install fledge` without `--locked`. Both must land on a patched release. The
caret requirement `49.0.1` admits nothing older than 49.0.1.

The committed lock moves with a targeted `cargo update -p wasmtime -p wasmtime-wasi`,
not a blanket update, so nothing outside the wasmtime tree changes.

## Map the preopen grants one to one

| Grant | Before | After | Meaning in both |
|---|---|---|---|
| `/project` | `DirPerms::READ`, `FilePerms::READ` | `FsPerms::ReadOnly` | read and list; no create, remove, rename, set-times or write-open |
| `/plugin` | `DirPerms::all()`, `FilePerms::all()` | `FsPerms::ReadWrite` | everything |

wasmtime-wasi 49 collapsed the two bitflags into one enum. fledge only ever used the
two corners of the old grid (read/read and all/all), and those are exactly the two
variants that remain. A comment in `build_wasi_p1` records the mapping.

## Keep the network grant's configuration identical

On 46.0.3 a `network` grant meant: TCP on, UDP on (both by default), every address
allowed (`inherit_network()`), IP name lookup off (by default). 49.0.1 turned TCP and
UDP off by default, so `inherit_network()` alone would leave a granted plugin with no
sockets. The grant now reads `inherit_network().allow_tcp(true).allow_udp(true)`, which
is the 46.0.3 configuration exactly. IP name lookup stays off, as before.

Without the grant, 49.0.1 now also has TCP and UDP off. On 46.0.3 they were on but
every address was refused, so no socket could connect or bind either way. This
change does not switch them back on for ungranted plugins. That would only loosen a
default for no gain.

None of this is observable from a preview 1 guest today, because preview 1 has no call
that creates a socket. The point is that the grant keeps meaning what it meant, so a
future move to preview 2 does not inherit a silent behaviour change.

## Pin the grants with tests that run real WASI calls

The sandbox had no test of the read-only and read-write split, which is the code this
change rewrites. The new tests in `src/plugin/wasm.rs` build the WASI context with
`build_wasi_p1` and the linker with `setup_linker`, the same calls `run_wasm_plugin`
makes, and drive `path_open` / `path_create_directory` from a WAT guest. They assert
both the WASI errno and the host filesystem (the file must not appear, the contents
must not change).

They call `build_wasi_p1` directly with a temporary project root instead of going
through `run_wasm_plugin`. `run_wasm_plugin` mounts the process's current directory
as `/project`, which in `cargo test` is the repository itself. A test that tries to
write there would, if the sandbox were broken, write into the checkout.

The memory cap test goes through `run_wasm_plugin`. The guest exits 0 only on the
expected `memory.grow` outcome and hits `unreachable` on the other. The plain error
text cannot carry the guest's exit code, because the top-level message drops the cause
chain.

## Declare the Windows feature fledge actually uses

`src/lanes/execute.rs` calls `CreateJobObjectW`, which windows-sys 0.59 compiles only
with `Win32_Security`. fledge had been getting that feature from wasmtime's dependency
tree. Declaring it in fledge's own `[target."cfg(windows)".dependencies]` makes the
Windows build independent of what wasmtime happens to pull in, and fixes
`--no-default-features` on Windows as a side effect. The alternative, pinning
`cap-primitives` 3 back into the graph, would be a dependency kept only for a feature
side effect.

## Keep the WebAssembly feature set at 46.0.3's

`create_engine` used to take wasmtime's default feature set. wasmtime 47 added GC,
exception handling and typed function references to it, and 49 added wide arithmetic,
so the upgrade alone would let a plugin use four proposals that 46.0.3 refused.
`create_engine` now turns all four off with `wasm_gc(false)`, `wasm_exceptions(false)`,
`wasm_function_references(false)` and `wasm_wide_arithmetic(false)`.

No plugin that ran on 1.8.0 can depend on them, since 46.0.3 rejected such modules at
compile time, and `wasm32-wasip1` toolchains do not emit them by default. They are
extra guest-reachable runtime, not just syntax: RUSTSEC-2026-0315, fixed in the same
49.0.1 release, let `call_ref` and exception `catch` drop fuel accounting. Turning
them on is a separate decision with its own review, not a side effect of a security
patch. `engine_keeps_wasmtime_46_feature_set` pins the set: a baseline module compiles,
and one module per proposal is refused.

## Let the cache invalidate itself

The `.cwasm` stamp's second line is `WASMTIME_DEP_VERSION`, which `build.rs` reads
from the `wasmtime` requirement in `Cargo.toml`. It changes from `46.0.1` to
`49.0.1`, so every installed plugin's cached module fails the stamp check and is
recompiled once, silently, on its next run. A 46.x `.cwasm` could not be deserialized
by 49.x anyway. No migration code is needed.
