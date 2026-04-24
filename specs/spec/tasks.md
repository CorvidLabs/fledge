---
spec: spec.spec.md
---

## Tasks

- [x] Write spec files for spec module
- [x] Implement SpecFrontmatter parsing (YAML frontmatter extraction)
- [x] Implement spec check: validate frontmatter, sections, file existence
- [x] Implement spec init: scaffold .specsync/ directory
- [x] Implement spec new: scaffold spec module directory
- [x] Wire up CLI subcommands in main.rs
- [x] Write unit tests
- [x] Run verification suite (test, clippy, fmt)
- [x] Implement `spec list` + `ls` alias with pretty and JSON output
- [x] Implement `spec show <name>` with pretty and JSON output
- [x] Share a `COMPANION_FILES` constant between check/list/show
- [x] Bump spec module to v2 and document new subcommands

## Gaps

- No way to filter `spec list` by status or companion-completeness — agents must post-filter JSON
- `spec show` does not include companion file *contents* — reading `tasks.md`/`context.md` still requires a filesystem read
- No `spec sections <name>` subcommand to dump section bodies (agents wanting the Purpose of a module must still parse the markdown themselves)
