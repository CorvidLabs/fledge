---
spec: prompts.spec.md
---

## User Stories

- As a user, I want to interactively choose a template from a list when I don't specify one
- As a user, I want my author name and GitHub org to auto-fill from config or git so I don't type them every time
- As a template author, I want to define custom prompts with optional defaults in `template.toml`
- As a template author, I want prompt defaults to reference previously collected variables

## Acceptance Criteria

- `select_template()` presents an interactive list with name and description columns
- `prompt_variables()` collects all core variables (project_name, author, github_org, license, year, date) and template-specific prompts
- Author falls back: config → `git config user.name` → interactive prompt
- GitHub org falls back: config → interactive prompt with "CorvidLabs" default
- License is always pulled from config (defaults to MIT)
- Template-specific prompt defaults support Tera variable interpolation
- Case conversion produces correct snake_case and PascalCase variants

## Constraints

- Must work in TTY environments (interactive prompts require a terminal)
- `dialoguer` prompts block on stdin — no async support needed
- Prompt ordering: core variables first, then template-specific prompts (so defaults can reference core vars)

## Out of Scope

- Non-interactive/batch mode for prompts (handled by `--yes` in init, not in prompts module)
- Prompt validation rules (type checking, regex patterns)
- Multi-select or boolean prompts — currently only text input and single-select
