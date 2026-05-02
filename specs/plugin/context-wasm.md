---
spec: plugin-wasm.spec.md
---

## Context

The canary plugin audit (fledge-plugin-canary v0.5.x) proved that the current plugin capability system only gates the fledge-v1 RPC protocol — the process itself runs as the user with full system access. A plugin with zero capabilities can still read `~/.ssh/`, steal GitHub tokens, inject git hooks, and exfiltrate data. WASM sandboxing is the structural fix: capabilities map to WASM imports that are linked or omitted at instantiation time, giving compile-time-like enforcement instead of runtime honor-system checks.

## Related Modules

- `plugin` — plugin resolution, manifest parsing, capability model (adds `runtime` field)
- `plugin-protocol` — fledge-v1 message types, reused over WASM imports instead of stdio
- `trust` — trust tier classification (native plugins flagged as elevated risk in 2.0)
- `config` — plugin directory paths, `.cwasm` cache location

## Design Decisions

- **Wasmtime with WASI P1** — battle-tested runtime used by Cloudflare Workers, Fastly, Fermyon. WASI P1 gives us filesystem preopens and sockets with broad language ecosystem support. A future migration to WASI P2 (component model) is possible once the ecosystem matures.
- **Capabilities as link-time enforcement** — ungranted capabilities mean the host import is never linked. If the plugin tries to call it, instantiation fails. This is structurally stronger than a runtime permission check.
- **Two new fine-grained capabilities** — `filesystem` (none/project/plugin) and `network` (bool) fill gaps the native model couldn't express (native always had full fs/network).
- **Additive in 1.1.0, default in 2.0.0** — no breaking changes for existing plugin authors. WASM is opt-in first, then becomes the encouraged path once the ecosystem matures.
- **Rust SDK crate (`fledge-plugin-sdk`)** — wraps raw WASM imports into the same ergonomic API plugin authors already know from the native protocol.
- **Module caching (`.cwasm`)** — Wasmtime's AOT compilation format. Eliminates cold-start compilation latency after first run.
