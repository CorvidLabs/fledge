---
change: CHG-0005-fix-parse-source-ref-rejecting-refs-containing-a-slash
artifact: tasks
---

# Tasks

- [x] Reproduce: `parse_source_ref("owner/rune@chore/0.2.0-launch-prep")` returns the whole string unsplit
- [x] Rewrite the credential-URL guard to check `@` position (inside the authority) instead of whether the ref contains `/`
- [x] Add tests for slash-containing refs (bare shorthand, full URL, and credentialed URL + trailing ref)
- [x] Update `specs/trust/trust.spec.md` (invariant 5 rewording, invariant 12, behavioral examples, changelog, version bump)
- [x] `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`
