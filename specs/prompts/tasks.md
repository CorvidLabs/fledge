---
spec: prompts.spec.md
---

## Tasks

- [x] Implement `select_template()` with dialoguer Select
- [x] Implement `prompt_variables()` collecting core and template-specific variables
- [x] Add author fallback chain (config → git → interactive)
- [x] Add github_org fallback (config → interactive with default)
- [x] Add `to_snake_case` and `to_pascal_case` helpers
- [x] Add `render_default` with Tera interpolation for prompt defaults
- [x] Add date/year automatic injection
- [x] Unit tests for case conversion, default rendering

## Gaps

- No tests for `select_template()` or `prompt_variables()` (require TTY interaction)
- No validation on prompted values (empty string accepted)

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
