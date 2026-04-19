---
spec: update.spec.md
---

## User Stories

- As a user, I want to run `fledge update` in my project to pull in template improvements
- As a user, I want to preview changes with `--dry-run` before applying
- As a user, I want my customized files to be preserved during updates
- As a user, I want new template files to be added automatically
- As a user, I want to be warned about files that were removed from the template

## Acceptance Criteria

- `fledge update` in a fledge-created project updates unmodified files from the latest template
- `fledge update --dry-run` shows what would change without writing
- User-modified files are skipped with a clear message
- New files from the template are added
- Removed template files produce a warning (not deleted)
- `.fledge.toml` is updated with new hashes after successful update
- Running `fledge update` when nothing changed prints "Already up to date"
- `fledge update --refresh` forces re-fetch of remote templates

## Constraints

- Must work with both built-in and remote templates
- Must not delete user files
- `.fledge.toml` must be human-readable

## Out of Scope

- Three-way merge for user-modified files (future v3)
- Interactive conflict resolution
- Partial updates (specific files only)
