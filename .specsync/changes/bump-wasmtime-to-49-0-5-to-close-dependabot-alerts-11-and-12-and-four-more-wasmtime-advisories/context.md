---
change: bump-wasmtime-to-49-0-5-to-close-dependabot-alerts-11-and-12-and-four-more-wasmtime-advisories
artifact: context
---

# Context

Dependabot raised two alerts on CorvidLabs/fledge against wasmtime >=49.0.0, <49.0.4, the optional `wasm` feature's
runtime (on by default):

- #12 (critical, GHSA-mghf-wjfp-p5xm / RUSTSEC-2026-0379): the Cranelift backend on aarch64 can jump to the wrong
  target on a large Wasm `br_table`.
- #11 (moderate, GHSA-9pvw-hwfg-92h5 / RUSTSEC-2026-0378): a panic lowering a host-created component-model stream into
  a guest that expects a different payload type.

Main asked for wasmtime and wasmtime-wasi 49.0.2 and locked 49.0.3. The task (from t38q at Leif's request,
2026-11-13) first scoped a bump to 49.0.4. A local `cargo audit` showed 49.0.4 clears both alerts but leaves four more
wasmtime advisories that only 49.0.5 fixes (RUSTSEC-2026-0420 wasmtime-wasi wasip3 UDP send, -0422 i31/nullref typed
function panic, -0423 cross-engine struct type confusion, -0424 unchecked memory in borrowed-resource transfer).
t38q then moved the scope to 49.0.5, still a patch-level bump of the same crate family.

Things a later session needs to know:

- `cargo update -p <crate> --precise` re-resolves an unsatisfied manifest requirement to the newest 49.x before
  applying `--precise`, so the lock is pinned first (manifest still at 49.0.2) and the manifest raised after.
- Each `cargo update` run also reassigns the `windows-sys` edge (0.52 / 0.59 / 0.61 are all in the lock) of crates
  that accept a range: errno, rustix, tempfile, winapi-util, fs-set-times, io-extras, winx. Those seven blocks were
  restored byte-for-byte from main so the lock diff is only the 32 family packages; `cargo metadata --locked` and
  `cargo tree --locked --target all` accept the result without rewriting it.
- build.rs exports the Cargo.toml wasmtime requirement as `WASMTIME_DEP_VERSION`, which src/plugin/wasm.rs writes
  into each `.cwasm` cache stamp. Raising the requirement changes the stamp from 49.0.2 to 49.0.5, so every cached
  plugin compiled by an earlier fledge (including by the affected aarch64 Cranelift) is recompiled on first run. That is
  the existing invalidation rule doing its job; the rule itself is unchanged.
- Not in this change: rustls-webpki 0.103.15 (RUSTSEC-2026-0382) and webpki-roots 1.0.9 (RUSTSEC-2026-0381) are still
  reported by `cargo audit`, as are the pre-existing unmaintained/unsound warnings (instant, number_prefix, rustix
  RUSTSEC-2026-0486, tokio RUSTSEC-2026-0386).
- The draft change `upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314` already on main is left alone.
