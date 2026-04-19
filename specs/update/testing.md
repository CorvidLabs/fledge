---
spec: update.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/update.rs` (inline) | Unit | ProjectMeta parsing, file hash computation, diff logic, action classification |

## Manual Testing

- [ ] `fledge init my-app --template rust-cli` then `fledge update` in the project
- [ ] Modify a file, then `fledge update` — verify it's skipped
- [ ] `fledge update --dry-run` shows changes without writing
- [ ] Add a new file to template, then `fledge update` — verify it's added
- [ ] `fledge update` when already up to date prints "Already up to date"

## Edge Cases & Boundary Conditions

| Scenario | Expected Behavior |
|----------|-------------------|
| No `.fledge.toml` | Error: "No .fledge.toml found. Was this project created with fledge?" |
| Corrupt `.fledge.toml` | Error with parse details |
| Template no longer exists | Error with template name |
| File in project but not in hash map | Treated as user-added, never touched |
| Empty project (all files deleted) | Re-creates all template files |
| `.fledge.toml` has unknown keys | Ignored (forward compatibility) |
