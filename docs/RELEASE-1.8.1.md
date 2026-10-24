# Fledge 1.8.1 release closeout

Status: draft preparation against main `85d7c4efbca79947afa359504b095c95c486b391`. This branch changes only documentation and its new draft lifecycle record.

## Mandatory security dependency

Release is held on [PR #538](https://github.com/CorvidLabs/fledge/pull/538). Main still declares the old Wasmtime 46 dependency and Rust 1.89 floor. This PR neither duplicates the upgrade nor claims its Wasmtime 49/Rust-floor work is merged. #538 needs its real owner definition/implementation reviews, native lifecycle completion and security/native checks before integration. Rebase this preparation after that integration and reread the final Rust floor and target compatibility.

## Candidate and version surfaces

Cargo.toml and flake.nix already agree at 1.8.1. Preserve that version while qualifying one eventual main-derived source. Published GitHub binaries and Homebrew are 1.7.2; crates.io currently serves 1.8.0. A previous green GitHub release is not evidence those channels converged. The existing changelog describes the merged hook/task-graph fixes and publishing support; it now marks 1.8.1 as unreleased instead of implying a completed dated release.

Open PRs #539 (channel documentation), #540 (isolated config tests), and #541 (secret masking) are not included in this branch. The owner must select them explicitly after #538 qualification; their own reviews/lifecycles remain separate. Rerun candidate qualification after any selected implementation changes.

## Ordered closeout

- [ ] Finish and integrate #538 without bypassing reviews, audit or native lifecycle.
- [ ] Select the remaining release contents explicitly and rebase this handoff onto the reviewed source.
- [ ] Confirm Cargo.lock package version, Cargo.toml, flake.nix, changelog and candidate binary identity all agree at 1.8.1.
- [ ] Run native pre-commit, verify-native, required template validation, security audit, Trust and release/platform qualification on the exact final source. Current documentation checks do not qualify the pending security implementation.
- [ ] Complete real owner review and native finalization before merge.
- [ ] Separately authorize the v1.8.1 tag. A v* push triggers Release: tests, five platform builds, GitHub assets/checksums and crates.io publication. It is not a documentation-only operation.
- [ ] Read crates.io back at 1.8.1; a successful cargo publish exit alone is insufficient.
- [ ] Observe Post-Release Formula Update using real uploaded checksums; verify Homebrew and GitHub/crates versions converge. No formula checksum can be filled before assets exist.

No dependencies, runtime source, release workflows, tags, package registries or Homebrew files are modified by this preparation. No release, merge, publication or deployment is authorized by this handoff.
