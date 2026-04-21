---
spec: deps.spec.md
---

## Key Decisions

- Parse lock files directly rather than invoking ecosystem CLIs for the basic dependency list -- this keeps `fledge deps` fast and offline
- Shell out for --outdated, --audit, and --licenses since each ecosystem has mature purpose-built tools
- Delegate project type detection to `run::detect_project_type` rather than duplicating detection logic
- Java dependency listing gracefully degrades with a message instead of erroring, since Gradle/Maven don't produce standard lock files
- pnpm detected but not parsed -- YAML parsing would add a dependency for one format

## Files to Read First

- `src/deps.rs` -- all parsing logic, ecosystem dispatch, and CLI output
- `src/run.rs` -- `detect_project_type()` used for ecosystem detection
- `specs/deps/deps.spec.md` -- formal API and invariants

## Current Status

- All 7 ecosystems implemented (Rust, Node/npm, Node/yarn, Go, Python/requirements, Python/pipfile, Python/poetry, Ruby)
- 12 unit tests covering all parsers and edge cases
- --json, --outdated, --audit, --licenses all wired up
- Spec at v1

## Notes

- Lock file parsing is format-specific and fragile to format changes (e.g., npm lockfile v4 could change structure)
- The unquote() helper is shared across Cargo.lock and poetry.lock parsers
- Dependencies are always sorted alphabetically regardless of lock file order
