# Plugin WASM Runtime — Requirements

## Functional Requirements

1. Load and execute WASM plugins via Wasmtime when `plugin.toml` declares `runtime = "wasm"`
2. Link host imports (`fledge::send`, `fledge::recv`, `fledge::exit`) for all WASM plugins regardless of capabilities
3. Conditionally link `fledge::exec` import only when `exec = true` in capabilities
4. Conditionally link `fledge::store_set` and `fledge::store_get` imports only when `store = true`
5. Conditionally link `fledge::metadata` import only when `metadata = true`
6. Configure WASI filesystem preopens based on `filesystem` capability: none, project (read-only), or plugin (project read-only + plugin dir read-write)
7. Enable WASI socket imports only when `network = true`
8. Fail instantiation with a clear error when a plugin imports a function that is not linked (capability denied)
9. Enforce resource limits: 256 MB memory, 10 billion fuel (instructions), 60 second wall-clock timeout, 1 MB stack
10. Pre-compile `.wasm` to `.cwasm` (Wasmtime AOT) on install and cache in the plugin directory
11. Invalidate `.cwasm` cache when the source `.wasm` file hash changes
12. Scaffold a Rust WASM plugin via `fledge plugins create <name> --wasm` with Cargo.toml, plugin.toml, src/lib.rs, and build hook
13. Show `(wasm)` or `(native)` runtime label in `fledge plugins list` and `fledge plugins audit`
14. Default `runtime` to `"native"` in 1.1.0 (backward-compatible) and to `"wasm"` in 2.0.0
15. In 2.0.0, require explicit `runtime = "native"` and user confirmation for native plugin installs

## Non-Functional Requirements

1. Cached WASM module startup must be under 50ms (no JIT compilation on cached path)
2. WASM host interface must preserve fledge-v1 protocol semantics — same message types, same validation rules
3. Plugin SDK crate (`fledge-plugin-sdk`) must compile to `wasm32-wasip2` target
4. Native plugins must be completely unaffected — zero behavioral changes for existing installations
5. `--json` flag must include runtime type in machine-parseable output for list/audit operations
