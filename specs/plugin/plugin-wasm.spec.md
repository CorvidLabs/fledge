---
module: plugin-wasm
version: 2
status: active
files:
  - src/plugin/wasm.rs
db_tables: []
depends_on:
  - plugin
  - specs/plugin/plugin-protocol.spec.md
  - trust
  - config
---

# Plugin WASM Runtime

## Purpose

Sandboxed WebAssembly runtime for fledge plugins. WASM plugins compile to `.wasm` modules and run inside a Wasmtime host with capability-mediated access — no filesystem, network, or exec unless the host explicitly provides it. This is the security boundary that native plugins lack: a WASM plugin with zero capabilities literally cannot read `~/.ssh/` because the host never exposes that import.

Ships in **fledge 1.1.0** as an additive runtime alongside native. Existing native plugins are unaffected. In **fledge 2.0.0**, native plugins require explicit `trust = "native"` and user confirmation; WASM becomes the default and recommended path.

## Roadmap

### 1.1.0 — Additive Foundation

- `runtime = "wasm"` in `plugin.toml` opts into the WASM executor
- Native plugins (`runtime = "native"`, the implicit default) continue unchanged
- WASM plugins are sandboxed — capabilities map to host-provided WASM imports
- `fledge plugins create --wasm` scaffolds a Rust WASM plugin targeting `wasm32-wasip1`
- Community plugin registry (future) requires WASM

### 2.0.0 — Native Becomes Opt-In

- Default runtime is `"wasm"` when `plugin.toml` declares `protocol = "fledge-v1"` and no explicit `runtime`
- Native plugins require `runtime = "native"` and a user confirmation prompt during install
- `fledge plugins audit` flags native plugins as elevated risk
- Deprecation warnings on native plugin install

## Public API

### Exported Functions

All functions are `pub(crate)` — visible within the crate but not public API:

| Export | Description |
|--------|-------------|
| `run_wasm_plugin` | Full lifecycle: load module, build WASI context, link host imports, run `_start`, enforce limits |
| `load_module` | Load a `.wasm` file, using `.cwasm` cache when valid (hash + version + tamper check) |
| `compile_and_cache` | Pre-compile a `.wasm` binary to `.cwasm` with a 3-line stamp file (wasm hash, wasmtime version, cwasm hash) |

Internal (not exported): `setup_linker` (links host imports based on capabilities), `HostState` (per-invocation state: WASI context, plugin info, pending responses, store limits).

## Manifest Changes

### plugin.toml

```toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
protocol = "fledge-v1"
runtime = "wasm"                    # new: "wasm" or "native" (default: "native" in 1.1, "wasm" in 2.0)

[[commands]]
name = "deploy"
binary = "target/wasm32-wasip1/release/fledge_deploy.wasm"  # .wasm file instead of native binary

[capabilities]
exec = true
store = true
metadata = false
filesystem = "project"              # new: "none", "project", "plugin" (see Filesystem Access)
network = false                     # new: allow outbound network (see Network Access)
```

New fields:

| Field | Values | Default | Description |
|-------|--------|---------|-------------|
| `runtime` | `"wasm"`, `"native"` | `"native"` (1.1), `"wasm"` (2.0) | Execution runtime |
| `capabilities.filesystem` | `"none"`, `"project"`, `"plugin"` | `"none"` | Filesystem scope |
| `capabilities.network` | `bool` | `false` | Outbound network access |

### Capability Mapping

Capabilities in WASM mode map directly to host-provided imports. If a capability is not granted, the corresponding WASM import is not linked — the plugin module fails to instantiate if it tries to import an unavailable function, giving a compile-time-like guarantee rather than a runtime check.

| Capability | What the host provides | Native equivalent |
|------------|----------------------|-------------------|
| `exec` | `fledge::exec(command, cwd, timeout) -> ExecResult` | Spawns subprocess via fledge |
| `store` | `fledge::store_set(key, value)`, `fledge::store_get(key) -> Option<value>` | Reads/writes `state.json` |
| `metadata` | `fledge::metadata(keys) -> JSON` | Project metadata, git info, env |
| `filesystem = "project"` | WASI preopened dir: project root (read-only) | N/A (native has full fs) |
| `filesystem = "plugin"` | WASI preopened dirs: project root (read-only) + plugin `data/` subdir (read-write) | N/A |
| `network` | WASI socket API for outbound connections | N/A (native has full network) |

With zero capabilities and `filesystem = "none"`, a WASM plugin can:
- Receive the `init` message (args, project context, plugin info)
- Send `output`, `log`, and `progress` messages
- Exit with a status code

**Limitation:** Interactive UI messages (`prompt`, `confirm`, `select`, `multi_select`) are not supported in WASM mode. The WASM runtime has no mechanism to send responses back to the plugin synchronously. Sending these message types produces a warning and the message is dropped. Plugins requiring user input should use native runtime or pre-collect input via args.

It **cannot**: read any file, write any file, execute any command, make network requests, or access environment variables beyond what `init` provides.

## WASM Host Interface

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

Uses WASI sockets for outbound TCP/UDP connections. No listening sockets (plugins cannot open servers). DNS resolution is provided by the host.

## WASM Runtime Details

### Engine

Wasmtime with the WASI preview 1 target. Plugins compile to WASI P1 modules using `cargo build --target wasm32-wasip1` (Rust), TinyGo, or any language that targets `wasm32-wasip1`.

### Resource Limits

| Resource | Limit | Rationale |
|----------|-------|-----------|
| Memory | 256 MB max | Prevents OOM from plugin bugs |
| Fuel (instructions) | 10 billion | ~10 seconds of compute; prevents infinite loops |
| Execution time | 60 seconds wall clock | Hard timeout independent of fuel |
| Stack size | 1 MB | Standard WASM stack |
| Instance count | 1 per plugin invocation | No fork-bombing |

Fuel is Wasmtime's instruction-counting mechanism. When fuel runs out, the plugin traps with a clear error message. The wall-clock timeout catches cases where the plugin is blocked on a host call (e.g., waiting for user input via `fledge::recv()`).

### Startup

1. Fledge reads `plugin.toml`, sees `runtime = "wasm"`
2. Loads the `.wasm` binary from the path in `[[commands]].binary`
3. Creates a Wasmtime `Engine` and `Store` with resource limits
4. Links host imports based on granted capabilities (ungranted = not linked)
5. Instantiates the WASM module — fails fast if plugin imports unavailable functions
6. Calls the module's `_start` export (WASI convention)
7. Sends `init` message via `fledge::recv()`
8. Plugin runs its logic using `fledge::send()` / `fledge::recv()`
9. Plugin calls `fledge::exit()` or returns from `_start`

### Caching

Compiled WASM modules are cached at `<plugin_dir>/<binary>.cwasm` (Wasmtime's ahead-of-time compiled format). A companion `.cwasm.sha256` stamp file stores three lines: the SHA-256 hash of the source `.wasm`, the Wasmtime major version, and the SHA-256 hash of the `.cwasm` itself. The cache is invalidated when any of these three checks fail — source change, Wasmtime upgrade, or `.cwasm` tampering. On deserialization failure (e.g., corrupt cache), the module is recompiled from source with a warning. This eliminates compilation latency on subsequent runs — startup should be <50ms for cached modules.

## Plugin Authoring

### Rust (recommended)

```bash
# Scaffold a WASM plugin
fledge plugins create my-plugin --wasm

# Build
cd my-plugin
cargo build --target wasm32-wasip1 --release

# Test locally
fledge plugins install ./my-plugin
fledge my-plugin
```

The scaffold generates:
- `Cargo.toml` with `wasm32-wasip1` target and `fledge-plugin-sdk` dependency
- `src/lib.rs` with a minimal plugin using the SDK
- `plugin.toml` with `runtime = "wasm"` and `protocol = "fledge-v1"`
- `build` hook: `cargo build --target wasm32-wasip1 --release`

### SDK (planned)

`fledge-plugin-sdk` is a planned Rust crate that wraps the raw WASM imports into ergonomic APIs:

```rust
use fledge_plugin_sdk::prelude::*;

#[fledge_plugin]
fn main(ctx: PluginContext) -> Result<()> {
    let target = ctx.prompt("Deploy target:")
        .default("staging")
        .validate(NonEmpty)
        .ask()?;

    if !ctx.confirm(&format!("Deploy to {target}?"))? {
        ctx.output("Cancelled.\n");
        return Ok(());
    }

    ctx.progress("Deploying", 0, 3);
    // ... deployment logic ...
    ctx.progress("Deploying", 3, 3);
    ctx.progress_done();

    ctx.output(&format!("Deployed to {target}\n"));
    Ok(())
}
```

The SDK will be published as a crate. Non-Rust authors use the raw WASM import interface directly.

### Other Languages

Any language that compiles to `wasm32-wasip1` can be used:

| Language | Toolchain | Notes |
|----------|-----------|-------|
| Rust | `cargo build --target wasm32-wasip1` | First-class support via SDK crate |
| Go | TinyGo | WASI P1 supported |
| C/C++ | wasi-sdk | Low-level, no SDK wrapper |
| AssemblyScript | asc | Compiles to WASI P1 |

## Install & Update Flow

### Install

`fledge plugins install owner/repo` clones the repo, reads `plugin.toml`:

- If `runtime = "wasm"`: runs build hook, validates `.wasm` file exists, pre-compiles to `.cwasm`, capability prompt (same as native), symlink is to a thin native shim that loads the WASM module
- If `runtime = "native"` (or omitted): existing behavior unchanged

### Update

`fledge plugins update` pulls and rebuilds. For WASM plugins, the `.cwasm` cache is invalidated and recompiled.

### Audit

`fledge plugins audit` shows the runtime type:

```
Plugin Security Audit

  * fledge-deploy v1.0.0 [official] (wasm)
    Source: CorvidLabs/fledge-plugin-deploy
    Runtime: wasm (sandboxed)
    Capabilities:
      * exec — can run shell commands (via host proxy)
      * filesystem — project root (read-only)
    Commands: deploy

  * fledge-stats v0.2.0 [unverified] (native)
    Source: someone/fledge-stats
    Runtime: native (unsandboxed — full system access)
    Capabilities: none (but process has full access)
    Commands: stats
    ! Warning: native plugin runs unsandboxed

  Summary: 2 plugin(s), 1 native (unsandboxed), 1 wasm (sandboxed)
```

## Security Model

### What WASM sandboxing guarantees

1. **No ambient filesystem access.** A WASM plugin cannot read `~/.ssh/`, `~/.aws/credentials`, shell history, or any file outside its preopened directories.
2. **No ambient network access.** Without `network = true`, the plugin has no socket imports — it cannot phone home or exfiltrate data.
3. **No process spawning.** Without `exec = true`, the plugin cannot run shell commands. Even with exec, commands are proxied through the host with the same cwd validation as native.
4. **No environment variable access.** The plugin sees only what the `init` message provides. No `$HOME`, `$PATH`, `$GITHUB_TOKEN`, etc.
5. **Resource-bounded.** Memory, CPU, and wall-clock time are all capped. A buggy or malicious plugin cannot OOM the host or spin forever.
6. **Capability enforcement is structural.** Capabilities are enforced at WASM link time — if the import isn't linked, the code can't call it. This is not a runtime check that could be bypassed.

### What WASM sandboxing does NOT guarantee

1. **Exec is still powerful.** A plugin with `exec = true` can run arbitrary commands as the user, same as native. The sandbox only helps when exec is denied.
2. **Network + exec = exfiltration.** A plugin with both capabilities can read files via exec and send them over the network. The sandbox limits the combination surface.
3. **Timing side channels.** WASM plugins can measure execution time and potentially infer information. This is a theoretical concern, not a practical one for CLI plugins.
4. **Host bugs.** If Wasmtime has a sandbox escape vulnerability, the isolation breaks. We depend on Wasmtime's security posture (which is excellent — it's used in Cloudflare Workers, Fastly, Fermyon, etc.).

## Invariants

1. WASM plugins run inside a Wasmtime sandbox with WASI preview 1
2. Capabilities map to WASM imports — ungranted capabilities are not linked, causing instantiation failure if the plugin tries to import them
3. The fledge-v1 protocol is preserved — same message types, same semantics, different transport (WASM imports vs stdio pipes)
4. `filesystem = "none"` means zero preopened directories — the plugin cannot read or write any file
5. `filesystem = "project"` preopens only the project root, read-only
6. `filesystem = "plugin"` preopens project root (read-only) and plugin `data/` subdir (read-write) — the full plugin dir is never writable
7. `network = false` means no socket imports — the plugin cannot make any network connections
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

## Behavioral Examples

### Scenario: Install a WASM plugin

- **Given** a plugin repo with `runtime = "wasm"` in plugin.toml
- **When** user runs `fledge plugins install owner/fledge-plugin-deploy`
- **Then** fledge clones, runs build hook, validates `.wasm` binary exists, pre-compiles to `.cwasm`, prompts for capabilities, installs

### Scenario: Zero-capability WASM plugin

- **Given** a WASM plugin with all capabilities `false` and `filesystem = "none"`
- **When** the plugin tries to read a file
- **Then** instantiation fails because WASI filesystem imports are not linked (no preopened dirs)

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

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| WASM binary not found | `.wasm` file missing after build | Error with build hint |
| Instantiation failed | Plugin imports a function not linked (capability denied) | Error listing which imports are missing and which capabilities would provide them |
| Fuel exhausted | Plugin exceeds instruction limit | Trap with "plugin exceeded compute limit" message |
| Memory limit | Plugin exceeds 256 MB | Trap with "plugin exceeded memory limit" message |
| Wall-clock timeout | Plugin exceeds 60 seconds | Kill with timeout error |
| Invalid WASM | Binary is not valid WebAssembly | Error with validation details |
| WASI incompatible | Module is not a valid WASI P1 module | Error suggesting recompile with `wasm32-wasip1` target |
| Path traversal | Plugin attempts `..` escape from preopened dir | WASI denies the open — no host-side check needed |
| Cache corrupt | `.cwasm` fails to deserialize | Re-compile from `.wasm`, warn user |
| Cache tampered | `.cwasm` SHA-256 doesn't match stamp file | Invalidate cache, re-compile from `.wasm` |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `wasmtime` | WASM engine, WASI implementation, fuel metering, preopened dirs |
| `plugin` | Plugin resolution, manifest parsing, capability model |
| `plugin-protocol` | Message types, protocol lifecycle |
| `config` | Plugin directory paths, cache directory |
| `trust` | Trust tier classification (native risk labeling) |

### Consumed By

| Module | What is used |
|--------|-------------|
| `plugin` | `run_plugin` dispatches to WASM executor when `runtime = "wasm"` |

## Migration Guide

### For plugin authors

1. Add `runtime = "wasm"` to `plugin.toml`
2. Set `[[commands]].binary` to the `.wasm` output path
3. If using Rust: add `fledge-plugin-sdk` dependency, compile with `cargo build --target wasm32-wasip1 --release`
4. If using the `exec` capability for file reads: switch to `filesystem = "project"` (faster, no subprocess overhead)
5. Test: `fledge plugins install ./my-plugin && fledge my-plugin`

### For users

No action needed in 1.1.0. WASM plugins are installed and run the same way — the only visible difference is `(wasm)` in `fledge plugins list` and `(sandboxed)` in `fledge plugins audit`.

In 2.0.0, installing native plugins will show a warning. Users can approve with `--trust-native` or by confirming the interactive prompt.

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-05-02 | Initial spec — WASM plugin runtime with Wasmtime, capability-mediated sandboxing, additive in 1.1.0, default in 2.0.0 |
| 2 | 2026-05-02 | Promoted to active. Updated Public API to match implementation (no `WasmRuntime` struct). `filesystem = "plugin"` preopens `data/` subdir (not full plugin dir). Cache uses 3-line stamp with cwasm tamper detection. Added invariants for path canonicalization, contextual JSON errors, thread joining, atomic ordering. |
