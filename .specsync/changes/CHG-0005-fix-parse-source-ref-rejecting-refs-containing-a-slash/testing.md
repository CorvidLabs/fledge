---
change: CHG-0005-fix-parse-source-ref-rejecting-refs-containing-a-slash
artifact: testing
---

# Testing

## REQ-trust-006: `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs, including refs containing `/`

- Automated: `src/trust.rs::tests::parse_source_ref_branch_with_slash` — `"someone/rune@chore/0.2.0-launch-prep"` splits into base `"someone/rune"` and ref `Some("chore/0.2.0-launch-prep")`
- Automated: `src/trust.rs::tests::parse_source_ref_full_url_branch_with_slash` — same, for a full `https://github.com/...` clone URL
- Automated: `src/trust.rs::tests::parse_source_ref_credential_url_with_branch_ref` — a credentialed URL (`https://user:token@...`) plus a trailing slash-containing ref still splits correctly, keeping credentials in the base
- Automated: `src/trust.rs::tests::parse_source_ref_credential_url_no_split` (existing, unchanged) — a credential URL with no ref suffix still returns `None` for the ref, the regression guard for the bug this check protects against

## Regression coverage

- `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` all pass
