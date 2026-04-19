---
spec: prompts.spec.md
---

## Key Decisions

- Uses `dialoguer` crate for interactive prompts with the `ColorfulTheme` for consistent terminal styling
- Core variables (`project_name`, `project_name_snake`, `project_name_pascal`, `year`, `date`, `author`, `github_org`, `license`) are always injected — templates don't need to prompt for these
- Author resolution: config → git config → interactive prompt (three-tier fallback)
- Template-specific prompts defined in `template.toml` under `[prompts.<key>]` are rendered after core variables, allowing defaults to reference earlier variables (e.g., `"A {{ project_name }} project"`)
- Prompt defaults support Tera rendering — if the default contains `{{`, it's rendered against the current context

## Files to Read First

- `src/prompts.rs` — template selector, variable prompting, case conversion helpers
- `specs/prompts/prompts.spec.md` — formal API and invariants

## Current Status

- Template selector, variable prompting, and case conversion all implemented
- Custom prompt defaults with Tera rendering working
- 12 unit tests covering case conversion, default rendering, and error cases

## Notes

- `to_snake_case` converts hyphens to underscores and lowercases; `to_pascal_case` splits on `-`/`_` and capitalizes each segment
- `render_default` short-circuits when no `{{` is present, avoiding unnecessary Tera parsing
