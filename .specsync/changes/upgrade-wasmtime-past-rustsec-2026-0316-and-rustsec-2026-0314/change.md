---
id: upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314
state: draft
type: bug_fix
base_commit: 85d7c4efbca79947afa359504b095c95c486b391
---

# Upgrade wasmtime past RUSTSEC-2026-0316 and RUSTSEC-2026-0314

## Intent

Upgrade wasmtime past RUSTSEC-2026-0316 and RUSTSEC-2026-0314

## Affected Canonical Specs

- `plugin-wasm`

## Acceptance Criteria

- cargo audit reports neither RUSTSEC-2026-0316 nor RUSTSEC-2026-0314 against the committed Cargo.lock or a fresh cargo generate-lockfile resolution (the form CI's audit job checks), with wasmtime, wasmtime-wasi, wiggle and every wasmtime-internal crate at 49.0.1; the plugin sandbox behaves as it did on wasmtime 46.0.3, so /project is read-only under both the project and plugin scopes (create, write-open, truncate and mkdir refused with EPERM), /plugin is read-write, .. cannot leave /plugin, no filesystem grant means no preopens (EBADF), memory.grow succeeds up to 256 MiB and is refused past it, fuel exhaustion and the epoch timeout still map to their own errors, a network grant keeps TCP and UDP on with IP name lookup off, and the engine refuses GC, exception-handling, typed function references and wide-arithmetic modules as 46.0.3 did; the new src/plugin/wasm.rs tests pass on 49.0.1 and give the same results on main's 46.0.3; CI is green with --locked on this PR (test and integration on ubuntu, macos and windows, lint, audit, spec-check, intent-check)

## No-spec Rationale

Not applicable
