---
spec: update.spec.md
---

## Tasks

- [x] Write spec files for update module
- [x] Add `.fledge.toml` generation to `fledge init`
- [x] Implement `ProjectMeta` struct and parsing
- [x] Implement file hash computation (SHA-256)
- [x] Implement diff logic (compare old hashes vs new template vs current files)
- [x] Implement `fledge update` command with dry-run support
- [x] Wire up CLI subcommand in `main.rs`
- [x] Unit tests for meta parsing, hashing, diff logic
- [x] Integration: init → modify → update cycle

## Gaps

- Three-way merge not yet implemented (planned for future version)
- No progress bar for large template updates

## Review Sign-offs

- **Product**: pending
- **QA**: pending
- **Design**: n/a
- **Dev**: pending
