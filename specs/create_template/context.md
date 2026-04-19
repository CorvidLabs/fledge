---
spec: create_template.spec.md
---

## Key Decisions

- Interactive prompts with sensible defaults for all fields — no required answers
- Generated `template.toml` uses TOML string formatting via `{:?}` for proper escaping
- Hooks and custom prompts are opt-in to keep the default scaffold minimal
- Includes both a `README.md` (author-facing docs) and `README.md.tera` (example rendered file)
- The `src/` directory is created as a hint but left empty — authors fill it with their template files

## Files to Read First

- `src/create_template.rs` — all scaffolding logic
- `src/main.rs` — `CreateTemplate` variant in `Commands` enum
- `specs/create_template/create_template.spec.md` — formal API and invariants

## Current Status

- Full implementation with interactive prompts (name, description, render globs, hooks, prompts)
- Generates valid `template.toml`, example `.tera` file, `.gitignore`, and author README
- 5 unit tests covering scaffold output, manifest validity, and error cases
- Spec at v1

## Notes

- The command is `fledge create-template <name>` (clap converts underscores to hyphens)
- Uses `dialoguer` for interactive prompts, consistent with the rest of the CLI
