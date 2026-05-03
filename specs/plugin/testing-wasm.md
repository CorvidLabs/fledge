---
spec: plugin-wasm.spec.md
---

## Test Plan

### Unit Tests

- Parse `runtime = "wasm"` from plugin.toml manifest
- Parse `filesystem` capability (none, project, plugin) and `network` capability (bool)
- Default `runtime` to `"native"` when field is absent
- `link_capabilities` links correct imports for each capability combination
- `link_capabilities` omits imports when capabilities are denied
- Resource limit configuration (memory, fuel, stack, timeout)
- `.cwasm` cache path generation from plugin name
- Cache invalidation when `.wasm` file hash changes

### Integration Tests

- Load and run a minimal WASM plugin (send/recv/exit only, zero capabilities)
- WASM plugin with `exec = true` can execute a command and receive stdout/stderr
- WASM plugin with `store = true` can set and get key-value pairs
- WASM plugin with `metadata = true` receives project metadata
- WASM plugin with `filesystem = "project"` can read files in project root
- WASM plugin with `filesystem = "project"` cannot write to project root
- WASM plugin with `filesystem = "plugin"` can read project and read-write plugin dir
- WASM plugin with `filesystem = "none"` cannot open any files
- WASM plugin with `network = false` cannot make outbound connections
- WASM plugin importing a denied capability fails at instantiation with clear error message
- Fuel exhaustion produces a trap with descriptive error
- Memory limit exceeded produces a trap with descriptive error
- Wall-clock timeout kills the plugin after 60 seconds
- Path traversal (`..`) from preopened directory is denied by WASI
- `.cwasm` cache is created on first run and reused on subsequent runs
- `fledge plugins create test-plugin --wasm` produces a valid scaffold that builds and runs
- `fledge plugins list` shows `(wasm)` for WASM plugins and `(native)` for native plugins
- `fledge plugins audit` shows sandboxed/unsandboxed labels correctly

### Canary Validation

- Port fledge-plugin-canary to WASM with zero capabilities
- Run baseline tests — all credential probes, file access, and persistence vectors must fail (0 warnings)
- Compare output with native canary (12+ warnings) to demonstrate sandbox effectiveness
