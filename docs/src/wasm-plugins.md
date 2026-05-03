# WASM Plugins

WASM plugins run in a sandboxed [Wasmtime](https://wasmtime.dev/) runtime with no host access by default. They're ideal for pure-computation tasks (linting, formatting, analysis, code generation) where you want strong isolation without trusting arbitrary native binaries.

## Quick start

```bash
fledge plugins create fledge-my-lint --wasm
cd fledge-my-lint
cargo build --target wasm32-wasip1 --release
fledge plugins validate
```

## How it works

When fledge runs a WASM plugin:

1. The `.wasm` binary is compiled to native code and cached as `.cwasm` (version-stamped, invalidated on Wasmtime upgrades)
2. A Wasmtime instance is created with the declared capabilities
3. The plugin communicates via the same [fledge-v1 protocol](./plugins.md#plugin-protocol-fledge-v1) as native plugins. JSON messages over host-provided `send`/`recv` functions
4. Execution is bounded by fuel (CPU) and a 60-second wall-clock timeout
5. Memory is capped at 256 MB

## Capabilities

WASM plugins declare capabilities in `plugin.toml`. All default to denied.

| Capability | Values | Effect |
|------------|--------|--------|
| `exec` | `true`/`false` | Execute shell commands on the host |
| `store` | `true`/`false` | Persist key-value data between runs |
| `metadata` | `true`/`false` | Read project metadata (language, name, git info) |
| `filesystem` | `"none"`, `"project"`, `"plugin"` | `"project"` mounts project root read-only. `"plugin"` adds a read-write plugin directory. |
| `network` | `true`/`false` | Inherit the host network stack |

Users are prompted to approve capabilities at install time. Denied capabilities fail gracefully at runtime. No crashes.

## plugin.toml for WASM

```toml
[plugin]
name = "fledge-my-lint"
version = "0.1.0"
description = "Custom linting rules"
protocol = "fledge-v1"
runtime = "wasm"

[[commands]]
name = "my-lint"
description = "Run custom lint rules"
binary = "target/wasm32-wasip1/release/fledge-my-lint.wasm"

[hooks]
build = "cargo build --target wasm32-wasip1 --release"

[capabilities]
filesystem = "project"
network = false
```

The key differences from native plugins:
- `runtime = "wasm"` tells fledge to use the Wasmtime sandbox
- `binary` points to a `.wasm` file instead of a native executable
- `filesystem` and `network` are WASM-specific capability fields

## Writing a WASM plugin in Rust

The scaffold (`fledge plugins create --wasm`) generates a working starter. The core pattern:

```rust
#[link(wasm_import_module = "fledge")]
extern "C" {
    fn send(ptr: *const u8, len: u32) -> i32;
    fn recv(ptr: *mut u8, len: u32) -> i32;
}

fn send_message(msg: &str) {
    let bytes = msg.as_bytes();
    unsafe { send(bytes.as_ptr(), bytes.len() as u32) };
}

fn recv_message(buf: &mut [u8]) -> Option<&str> {
    let n = unsafe { recv(buf.as_mut_ptr(), buf.len() as u32) };
    if n <= 0 { return None; }
    std::str::from_utf8(&buf[..n as usize]).ok()
}

fn main() {
    // Read init message
    let mut buf = vec![0u8; 65536];
    let init = recv_message(&mut buf);

    // Do your work...

    // Send output
    send_message(r#"{"type":"output","data":"Lint passed!"}"#);
}
```

## Building

```bash
# One-time setup
rustup target add wasm32-wasip1

# Build
cargo build --target wasm32-wasip1 --release
```

The `[hooks] build` field in `plugin.toml` runs this automatically during `fledge plugins install`.

## Testing locally

```bash
# Validate manifest and binary
fledge plugins validate

# Install from local directory (push to GitHub first, or copy manually)
cp -r . ~/Library/Application\ Support/fledge/plugins/fledge-my-lint/

# Run it
fledge plugins run my-lint
```

## Resource limits

| Resource | Limit | Behavior on exceed |
|----------|-------|--------------------|
| CPU | Fuel-bounded | Plugin traps, host continues |
| Wall clock | 60 seconds | Plugin traps, host continues |
| Memory | 256 MB | Allocation fails, plugin traps |
| Stack | Wasmtime default | Plugin traps on overflow |

All limits result in a clean trap. The host process never crashes or enters an invalid state.

## When to use WASM vs native

| Use WASM when... | Use native when... |
|---|---|
| Pure computation over project files | Need to shell out to external tools |
| Untrusted or community plugins | Need unrestricted filesystem access |
| Cross-platform distribution (single binary) | Need pipes, redirects, or shell features |
| You want capability enforcement | Performance-critical host integrations |

## Security model

- **No ambient authority**: WASM plugins start with zero access. Every capability is opt-in and user-approved.
- **Memory isolation**: Guest memory is separate from host memory. Out-of-bounds access traps the plugin.
- **Deterministic execution**: Same inputs produce same outputs (no access to system clock, random, etc. unless granted).
- **Cache integrity**: Compiled `.cwasm` files are SHA-256 verified and version-stamped. Corruption or Wasmtime version mismatch triggers recompilation.

## Limitations

- No interactive UI (prompt/confirm/select). WASM plugins must use non-interactive output
- No direct access to environment variables or the host filesystem beyond declared mounts
- Build requires the `wasm32-wasip1` target (`rustup target add wasm32-wasip1`)
- Currently Rust-only for the scaffold; any language that compiles to WASI works in practice
