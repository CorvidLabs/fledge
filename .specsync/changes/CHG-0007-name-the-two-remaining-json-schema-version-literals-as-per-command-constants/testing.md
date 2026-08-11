---
change: CHG-0007-name-the-two-remaining-json-schema-version-literals-as-per-command-constants
artifact: testing
---

# Testing

## Automated

- `cargo test` — full suite, including the two new `envelope::tests` cases:
  - `resource` output is byte-identical to the equivalent hand-rolled `json!`.
  - `versioned(v, to_value(struct))` flattens the struct's fields to the top level
    beside `schema_version` rather than nesting them.
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`

## Manual wire-format check

`fledge plugins validate --json` and `fledge lanes validate --json` were run against
the pre-change and post-change binaries and the output compared. Both are byte-identical,
which is the invariant this change must not break.

## Rejection signal

If either validate command's `--json` output differs in any byte from the pre-change
binary, or if `schema_version` reports anything other than `1` for either command,
the change is wrong and must not be accepted.
