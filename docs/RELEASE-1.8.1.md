# Fledge 1.8.1 release closeout

Status: draft preparation, rebased onto main after the security upgrade. This branch changes only documentation and its lifecycle record. No release, tag, merge, registry publication or deployment is authorized by this handoff; cutting 1.8.1 is Leif's call.

## Security dependency: resolved

The release hold is cleared. #538 upgraded wasmtime/wasmtime-wasi 46.0.3 to 49.0.1 (locked at 49.0.3), clearing RUSTSEC-2026-0316 and RUSTSEC-2026-0314, and #543 dropped wasmtime's unused compilation-cache feature so zstd left the dependency graph, clearing RUSTSEC-2026-0391, RUSTSEC-2026-0398 and RUSTSEC-2026-0403. The hosted `audit` job is green on main. Building from source now needs Rust 1.96+ (wasmtime 49's floor).

One residual process note: #538's native change (`upgrade-wasmtime-past-rustsec-2026-0316-and-rustsec-2026-0314`) still sits as an active draft in `.specsync/changes/` on main; its archive evidence was never finalized. Closing that record is a bookkeeping task, not a code gate.

## Contents of 1.8.1

Everything in the changelog's v1.8.1 section, which is main plus this documentation. Since the 1.8.0 crates.io cut, that adds #531, #533, #534, #535, #538, #543, and:

- #541 — `config set` masks recognized secrets in its confirmation (`***` instead of the value; the value is still saved). Security fix.
- #540 — config init/command tests are isolated from the operator's HOME and config. Test fix; no production behavior change.

## Candidate and version surfaces

Cargo.toml, Cargo.lock and flake.nix agree at 1.8.1. crates.io serves 1.8.0; GitHub releases and Homebrew serve 1.7.2. A previous green GitHub release is not evidence those channels converged.

## Ordered closeout

- [x] Integrate the Wasmtime security upgrade (#538) and the zstd cleanup (#543) with a green hosted audit.
- [x] Select the remaining release contents explicitly (#540, #541 merged after their own lifecycle and hosted checks).
- [ ] Confirm Cargo.lock package version, Cargo.toml, flake.nix, changelog and candidate binary identity all agree at 1.8.1 on the exact final source.
- [ ] Run native pre-commit, verify-native, required template validation, security audit, Trust and release/platform qualification on the exact final source.
- [ ] Complete real owner review of this handoff.
- [ ] Separately authorize the v1.8.1 tag. A v* push triggers Release: tests, five platform builds, GitHub assets/checksums and crates.io publication. It is not a documentation-only operation.
- [ ] Read crates.io back at 1.8.1; a successful cargo publish exit alone is insufficient.
- [ ] Observe Post-Release Formula Update using real uploaded checksums; verify Homebrew and GitHub/crates versions converge. No formula checksum can be filled before assets exist.

No dependencies, runtime source, release workflows, tags, package registries or Homebrew files are modified by this preparation.
