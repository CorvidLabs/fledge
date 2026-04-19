---
spec: update.spec.md
---

## Tasks

- [x] Write spec files for update module
- [ ] Add `.fledge.toml` generation to `fledge init`
- [ ] Implement `ProjectMeta` struct and parsing
- [ ] Implement file hash computation (SHA-256)
- [ ] Implement diff logic (compare old hashes vs new template vs current files)
- [ ] Implement `fledge update` command with dry-run support
- [ ] Wire up CLI subcommand in `main.rs`
- [ ] Unit tests for meta parsing, hashing, diff logic
- [ ] Integration: init → modify → update cycle

## Gaps

- Three-way merge not yet implemented (planned for future version)
- No progress bar for large template updates

## Review Sign-offs

- **Product**: pending
- **QA**: pending
- **Design**: n/a
- **Dev**: pending
