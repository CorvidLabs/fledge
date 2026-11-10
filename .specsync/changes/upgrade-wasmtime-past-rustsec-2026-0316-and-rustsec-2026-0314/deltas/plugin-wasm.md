---
change: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
module: plugin-wasm
---

## ADDED

### REQUIREMENT REQ-plugin-wasm-001

Under `filesystem = "project"` and `filesystem = "plugin"`, the host SHALL mount the project root as `/project` read-only (`FsPerms::ReadOnly`): a plugin can open files there for reading, and creating a file, opening an existing file for writing (with or without `O_TRUNC`) and creating a directory are refused with `EPERM`, leaving the host filesystem unchanged.

Acceptance Criteria
- Under `filesystem = "project"`, `path_open` for reading an existing file under `/project` returns 0.
- Under `filesystem = "project"`, `path_open` with `O_CREAT` under `/project` returns `EPERM` (63) and no file appears in the project root.
- Under `filesystem = "project"`, `path_open` for writing an existing file, with and without `O_TRUNC`, returns `EPERM` and the file's contents are unchanged.
- Under `filesystem = "project"`, `path_create_directory` under `/project` returns `EPERM` and no directory appears.
- Under `filesystem = "plugin"`, `path_open` with `O_CREAT` under `/project` returns `EPERM` and no file appears.

### REQUIREMENT REQ-plugin-wasm-002

Under `filesystem = "plugin"`, the host SHALL create `<plugin_dir>/data/` and mount it as `/plugin` read-write (`FsPerms::ReadWrite`), so a plugin can create files and directories there. A path that uses `..` SHALL NOT reach outside `<plugin_dir>/data/`, so the rest of the plugin directory (`plugin.toml`, the `.wasm` and `.cwasm` files) is never writable.

Acceptance Criteria
- Running a plugin with `filesystem = "plugin"` creates `<plugin_dir>/data/`.
- `path_open` with `O_CREAT` under `/plugin` returns 0 and the file appears in `<plugin_dir>/data/`.
- `path_create_directory` under `/plugin` returns 0 and the directory appears in `<plugin_dir>/data/`.
- `path_open` of `../escape.txt` with `O_CREAT` from `/plugin` fails, and no `escape.txt` appears in `<plugin_dir>`.

### REQUIREMENT REQ-plugin-wasm-003

When `filesystem` is absent or `"none"`, the host SHALL preopen no directory. The WASI preview 1 imports are still linked, so the plugin instantiates, but it holds no directory descriptor: descriptor 3, where the first preopen would sit, is not open.

Acceptance Criteria
- With default capabilities, a module importing `wasi_snapshot_preview1` `path_open` instantiates, and `path_open` on descriptor 3 returns `EBADF` (8).

### REQUIREMENT REQ-plugin-wasm-004

The host SHALL enable WASI TCP and UDP, with every socket address allowed, only for a plugin granted `network = true`; without the grant, TCP and UDP stay disabled and every socket address is refused. IP name lookup SHALL stay disabled with or without the grant. WASI preview 1, which fledge links, has no call that creates a socket and the host preopens none, so under preview 1 no plugin can open a connection either way; this configuration is what the grant means to a future preview 2 host.

Acceptance Criteria
- `build_wasi_p1` calls `inherit_network()`, `allow_tcp(true)` and `allow_udp(true)` only when `capabilities.network` is true.
- Apart from those three calls, `build_wasi_p1` configures no socket setting (no `allow_ip_name_lookup`, no `socket_addr_check` of its own), so wasmtime-wasi 49.0.1's builder defaults (TCP, UDP and IP name lookup off, every socket address refused) hold for an ungranted plugin, and IP name lookup stays off for a granted one.

### REQUIREMENT REQ-plugin-wasm-005

Every plugin store SHALL cap each linear memory at `MAX_MEMORY_BYTES`, 256 MiB (4096 pages of 64 KiB), through wasmtime's `StoreLimits`. `memory.grow` up to exactly the cap succeeds, and a grow past it returns -1 without trapping, so the plugin keeps running.

Acceptance Criteria
- Through `run_wasm_plugin`, a guest that starts with one page and grows by 4095 pages, to exactly 256 MiB, gets a result other than -1.
- Through `run_wasm_plugin`, a guest that starts with one page and grows by 4096 pages, one page past the cap, gets -1 and the run ends without a trap.

### REQUIREMENT REQ-plugin-wasm-006

The engine fledge compiles and runs plugins with (`create_engine`, used by both `compile_and_cache` and `run_wasm_plugin`) SHALL hold the WebAssembly feature set to the wasmtime 46.0.3 baseline: it SHALL refuse modules that use the GC, exception-handling, typed function references or wide-arithmetic proposals, which wasmtime 46.0.3 refused and wasmtime 47 and 49 turned on by default, and SHALL keep accepting what `wasm32-wasip1` toolchains emit by default (bulk memory, sign extension, saturating float-to-int conversion, multi-value, reference types) plus tail calls.

Acceptance Criteria
- A module using `memory.copy`, `i32.extend8_s`, `i32.trunc_sat_f32_s`, a multi-value function, `ref.null extern` with `ref.is_null`, and `return_call` compiles with `create_engine()`.
- A module using `array.new_default` (GC), one using `try_table` and `throw` (exception handling), one using `call_ref` (typed function references) and one using `i64.add128` (wide arithmetic) each fail to compile with `create_engine()`.

## MODIFIED

### SPEC SECTION WASM Host Interface

The host exposes functions in the `fledge` namespace that WASM plugins import. These are the **only** way for a WASM plugin to interact with the system.

### Core (always available)

```wit
// Plugin receives messages from fledge (init, response, cancel)
fledge::recv() -> Message

// Plugin sends messages to fledge (prompt, confirm, output, log, progress, etc.)
fledge::send(message: Message)

// Exit with status code
fledge::exit(code: u32)
```

These three functions are the WASM equivalent of stdin/stdout in the native protocol. The fledge-v1 JSON-lines protocol is preserved — `send` and `recv` serialize/deserialize the same message types. This means a plugin's protocol logic is identical whether it's native or WASM; only the I/O transport changes.

### Exec (requires `exec = true`)

```wit
fledge::exec(command: string, cwd: option<string>, timeout: option<u32>) -> ExecResult

record ExecResult {
    code: u32,
    stdout: string,
    stderr: string,
}
```

Identical semantics to the native `exec` protocol message. The host validates `cwd` and runs the command as a subprocess.

### Store (requires `store = true`)

```wit
fledge::store_set(key: string, value: string)
fledge::store_get(key: string) -> option<string>
```

Same limits as native: 256-byte keys, 64KB values, 1MB total, 256 keys max.

### Metadata (requires `metadata = true`)

```wit
fledge::metadata(keys: list<string>) -> string  // JSON-encoded object
```

Returns the same metadata as the native `metadata` protocol message.

### Filesystem (requires `filesystem != "none"`)

No custom imports needed — uses standard WASI filesystem preopens:

| `filesystem` value | Preopened directories |
|-------------------|----------------------|
| `"none"` | (no preopens) |
| `"project"` | Project root → `/project` (read-only) |
| `"plugin"` | Project root → `/project` (read-only), Plugin `data/` subdir → `/plugin` (read-write) |

Plugins see a virtual filesystem rooted at `/project` and `/plugin`. The `/plugin` mount points to `<plugin_dir>/data/`, not the full plugin directory — this prevents plugins from modifying their own `plugin.toml`, `.wasm`, or `.cwasm` files. All preopened paths are canonicalized before mounting to prevent symlink escapes. No access to home directories, system files, or other plugins' storage.

### Network (requires `network = true`)

The grant enables WASI TCP and UDP with every socket address allowed. IP name lookup (DNS) stays off with or without the grant, and without the grant TCP and UDP are disabled and every socket address is refused. fledge links WASI preview 1, which has no call that creates a socket, and the host preopens none, so under preview 1 a plugin cannot open a connection with or without the grant. The grant's configuration is what a future preview 2 host would inherit.

### SPEC SECTION Security Model

### What WASM sandboxing guarantees

1. **No ambient filesystem access.** A WASM plugin cannot read `~/.ssh/`, `~/.aws/credentials`, shell history, or any file outside its preopened directories.
2. **No ambient network access.** Without `network = true`, WASI TCP and UDP are disabled and every socket address is refused — the plugin cannot phone home or exfiltrate data over the network. The preview 1 socket imports are still linked; the WASI context, not the linker, refuses them.
3. **No process spawning.** Without `exec = true`, the plugin cannot run shell commands. Even with exec, commands are proxied through the host with the same cwd validation as native.
4. **No environment variable access.** The plugin sees only what the `init` message provides. No `$HOME`, `$PATH`, `$GITHUB_TOKEN`, etc.
5. **Resource-bounded.** Memory, CPU, and wall-clock time are all capped. A buggy or malicious plugin cannot OOM the host or spin forever.
6. **Capability enforcement is structural.** Capabilities are enforced at WASM link time — if the import isn't linked, the code can't call it. This is not a runtime check that could be bypassed.

### What WASM sandboxing does NOT guarantee

1. **Exec is still powerful.** A plugin with `exec = true` can run arbitrary commands as the user, same as native. The sandbox only helps when exec is denied.
2. **Network + exec = exfiltration.** A plugin with both capabilities can read files via exec and send them over the network. The sandbox limits the combination surface.
3. **Timing side channels.** WASM plugins can measure execution time and potentially infer information. This is a theoretical concern, not a practical one for CLI plugins.
4. **Host bugs.** If Wasmtime has a sandbox escape vulnerability, the isolation breaks. We depend on Wasmtime's security posture (which is excellent — it's used in Cloudflare Workers, Fastly, Fermyon, etc.).

### SPEC SECTION Invariants

1. WASM plugins run inside a Wasmtime sandbox with WASI preview 1
2. Capabilities map to WASM imports — ungranted capabilities are not linked, causing instantiation failure if the plugin tries to import them
3. The fledge-v1 protocol is preserved — same message types, same semantics, different transport (WASM imports vs stdio pipes)
4. `filesystem = "none"` means zero preopened directories — the plugin cannot read or write any file
5. `filesystem = "project"` preopens only the project root, read-only
6. `filesystem = "plugin"` preopens project root (read-only) and plugin `data/` subdir (read-write) — the full plugin dir is never writable
7. `network = false` means WASI TCP and UDP are disabled and every socket address is refused — the plugin cannot make any network connections, although the preview 1 socket imports are still linked. `network = true` enables TCP and UDP for every address. IP name lookup stays off either way
8. Resource limits (memory, fuel, wall-clock) are enforced by Wasmtime and cannot be disabled by plugins
9. Compiled WASM modules are cached as `.cwasm` with a 3-line stamp file — cache is invalidated by source `.wasm` hash change, wasmtime version mismatch, or `.cwasm` tamper (hash mismatch)
10. WASM plugins do not inherit host stderr — diagnostic output must use `fledge::send` with `Log` messages
11. Interactive UI messages (prompt/confirm/select) are rejected in WASM mode with a warning
12. Native plugins are completely unaffected by the WASM runtime addition (backward-compatible)
13. The `fledge-plugin-sdk` crate abstracts WASM imports into the same ergonomic API as the native protocol
14. In 2.0.0, installing a native plugin displays a warning and requires explicit user confirmation
15. All preopened paths are canonicalized before mounting to prevent symlink escapes
16. Host function JSON parse errors include the function name as context prefix (e.g., `"exec: malformed JSON: ..."`)
17. The timeout thread is joined after `_start` completes to prevent use-after-drop
18. Atomic ordering uses `Acquire` on reads and `Release` on stores for the finished flag

### SPEC SECTION Behavioral Examples

### Scenario: Install a WASM plugin

- **Given** a plugin repo with `runtime = "wasm"` in plugin.toml
- **When** user runs `fledge plugins install owner/fledge-plugin-deploy`
- **Then** fledge clones, runs build hook, validates `.wasm` binary exists, pre-compiles to `.cwasm`, prompts for capabilities, installs

### Scenario: Zero-capability WASM plugin

- **Given** a WASM plugin with all capabilities `false` and `filesystem = "none"`
- **When** the plugin tries to read a file
- **Then** the read fails: the WASI imports are linked, so the plugin instantiates, but no directory is preopened, so it holds no descriptor to resolve a path against (a raw `path_open` on descriptor 3 returns `EBADF`)

### Scenario: WASM plugin with filesystem = "project"

- **Given** a WASM plugin with `filesystem = "project"`
- **When** the plugin opens `/project/src/main.rs`
- **Then** read succeeds (project root is preopened read-only)
- **When** the plugin tries to open `/project/../.ssh/id_ed25519`
- **Then** open fails — WASI path resolution prevents directory traversal above the preopen

### Scenario: Native plugin unchanged

- **Given** an existing native plugin with no `runtime` field
- **When** user updates to fledge 1.1.0
- **Then** plugin continues to work exactly as before — `runtime` defaults to `"native"`

### Scenario: Canary plugin as WASM

- **Given** the fledge-plugin-canary ported to WASM with zero capabilities
- **When** `fledge canary` runs the baseline tests
- **Then** every file access, credential probe, and persistence vector check fails — the WASM sandbox prevents all of them
- **Then** output shows 0 warnings (vs 12+ warnings in native mode), proving the sandbox works

### SPEC SECTION Error Cases

| Error | When | Behavior |
|-------|------|----------|
| WASM binary not found | `.wasm` file missing after build | Error with build hint |
| Instantiation failed | Plugin imports a function not linked (capability denied) | Error listing which imports are missing and which capabilities would provide them |
| Fuel exhausted | Plugin exceeds instruction limit | Trap with "plugin exceeded compute limit" message |
| Memory limit | Plugin grows a linear memory past 256 MiB | `memory.grow` returns -1 instead of trapping and the plugin keeps running; a plugin that cannot handle the failed grow (a Rust allocator aborting, say) then traps, reported as `Plugin '<name>' trapped: …` |
| Wall-clock timeout | Plugin exceeds 60 seconds | Kill with timeout error |
| Invalid WASM | Binary is not valid WebAssembly | Error with validation details |
| WASI incompatible | Module is not a valid WASI P1 module | Error suggesting recompile with `wasm32-wasip1` target |
| Path traversal | Plugin attempts `..` escape from preopened dir | WASI denies the open — no host-side check needed |
| Cache corrupt | `.cwasm` fails to deserialize | Re-compile from `.wasm`, warn user |
| Cache tampered | `.cwasm` SHA-256 doesn't match stamp file | Invalidate cache, re-compile from `.wasm` |
