---
spec: plugin-wasm.spec.md
---

## Tasks

- [x] Write spec and companion files for WASM plugin runtime
- [x] Add `wasmtime` and `wasmtime-wasi` crate dependencies to Cargo.toml
- [x] Add `runtime` field to plugin manifest parser (`plugin.toml`)
- [x] Add `filesystem` and `network` capability fields to manifest parser
- [x] Implement `WasmRuntime` struct (Wasmtime engine, store, resource limits)
- [x] Implement `link_capabilities` — conditionally link host imports based on granted capabilities
- [x] Implement core host imports: `fledge::send`, `fledge::recv`, `fledge::exit`
- [x] Implement `fledge::exec` host import (proxied exec with cwd validation)
- [x] Implement `fledge::store_set` / `fledge::store_get` host imports
- [x] Implement `fledge::metadata` host import
- [x] Implement WASI filesystem preopens (none / project / plugin)
- [x] Implement WASI network (outbound sockets when `network = true`)
- [x] Implement `compile_and_cache` — AOT compile `.wasm` → `.cwasm` with hash-based invalidation
- [x] Implement `run_wasm_plugin` — full lifecycle from load to exit
- [x] Wire WASM executor into `plugin/mod.rs` dispatch (route based on `runtime` field)
- [x] Add `--wasm` flag to `fledge plugins create` scaffold command
- [x] Create WASM plugin scaffold template (Cargo.toml, plugin.toml, src/lib.rs, build hook)
- [x] Add runtime label to `fledge plugins list` and `fledge plugins audit` output
- [ ] Create `fledge-plugin-sdk` crate with `#[fledge_plugin]` macro and ergonomic API
- [ ] Port fledge-plugin-canary to WASM as validation test case
- [x] Run verification suite (spec check, clippy, tests)
