---
spec: update.spec.md
---

## Key Decisions

- v1 approach: auto-update unmodified files, skip user-modified files. No three-way merge yet.
- File modification detection uses SHA-256 hashes stored in `.fledge.toml` at init time
- `.fledge.toml` is written by `fledge init` and updated by `fledge update`
- Deleted template files produce warnings but are never auto-deleted (user may have added content)
- Template variables are re-used from the original init — no re-prompting
- Remote templates are re-fetched from the same source ref unless a newer version is available

## Files to Read First

- `src/update.rs` — update logic: load meta, diff, apply
- `src/init.rs` — where `.fledge.toml` is generated during init
- `specs/update/update.spec.md` — formal API and invariants

## Current Status

- v1 implementation: auto-update unmodified files, skip modified, add new, warn on deleted
- Future: three-way merge for user-modified files (v3)

## Notes

- `.fledge.toml` format is designed for forward compatibility — unknown keys are ignored
- Hash comparison uses SHA-256 for collision resistance
- The `[files]` section in `.fledge.toml` maps relative paths to their hashes at creation/update time
