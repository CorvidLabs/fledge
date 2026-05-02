---
spec: plugin-wasm.spec.md
---

## Tasks

- [x] Write spec and companion files for WASM plugin runtime
- [ ] Add `wasmtime` and `wasmtime-wasi` crate dependencies to Cargo.toml
- [ ] Add `runtime` field to plugin manifest parser (`plugin.toml`)
- [ ] Add `filesystem` and `network` capability fields to manifest parser
- [ ] Implement `WasmRuntime` struct (Wasmtime engine, store, resource limits)
- [ ] Implement `link_capabilities` — conditionally link host imports based on granted capabilities
- [ ] Implement core host imports: `fledge::send`, `fledge::recv`, `fledge::exit`
- [ ] Implement `fledge::exec` host import (proxied exec with cwd validation)
- [ ] Implement `fledge::store_set` / `fledge::store_get` host imports
- [ ] Implement `fledge::metadata` host import
- [ ] Implement WASI filesystem preopens (none / project / plugin)
- [ ] Implement WASI network (outbound sockets when `network = true`)
- [ ] Implement `compile_and_cache` — AOT compile `.wasm` → `.cwasm` with hash-based invalidation
- [ ] Implement `run_wasm_plugin` — full lifecycle from load to exit
- [ ] Wire WASM executor into `plugin/mod.rs` dispatch (route based on `runtime` field)
- [ ] Add `--wasm` flag to `fledge plugins create` scaffold command
- [ ] Create WASM plugin scaffold template (Cargo.toml, plugin.toml, src/lib.rs, build hook)
- [ ] Add runtime label to `fledge plugins list` and `fledge plugins audit` output
- [ ] Create `fledge-plugin-sdk` crate with `#[fledge_plugin]` macro and ergonomic API
- [ ] Port fledge-plugin-canary to WASM as validation test case
- [ ] Run verification suite (spec check, clippy, tests)
