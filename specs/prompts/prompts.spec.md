---
module: prompts
version: 3
status: active
files:
  - src/prompts.rs

db_tables: []
depends_on:
  - templates
---

# Prompts

## Purpose

Interactive user prompts using dialoguer. Handles template selection and variable collection (author, GitHub org, description, and template-specific custom prompts).

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `select_template` | Presents an interactive menu for the user to pick a project template |
| `prompt_variables` | Collects all template variables via config defaults, git, and interactive prompts |

### Structs & Enums

| Type | Description |
|------|-------------|

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `select_template` | `(&[Template]) -> Result<usize>` | Interactive template selection menu |
| `prompt_variables` | `(&Template, &str, &Config, bool, Option<&str>, Option<&str>) -> Result<tera::Context>` | Collects all template variables via prompts; bool is `yes` flag, last two are `author_override` and `org_override` from CLI flags |

## Invariants

1. Core variables (project_name, snake/pascal variants, year, date) are always set without prompting
2. Author uses CLI override → config → git, falling back to interactive prompt
3. GitHub org uses CLI override → config, defaulting to "CorvidLabs"
4. License is always read from config (no interactive prompt)
5. Template-specific prompts support Tera expressions in default values
6. Template-specific prompts are processed in manifest iteration order; earlier prompt values are available as Tera context for later prompt defaults

## Behavioral Examples

### Scenario: Author from git config

- **Given** no author in config, but `git config user.name` returns "Leif"
- **When** `prompt_variables()` is called
- **Then** author is set to "Leif" without prompting

### Scenario: Template-specific prompt with default

- **Given** template defines `description` prompt with default "A new Rust CLI"
- **When** user presses Enter without typing
- **Then** description is set to "A new Rust CLI"

### Scenario: Tera expression in prompt default

- **Given** template defines a prompt with default `"{{ project_name }} library"`
- **When** project_name is "my-tool"
- **Then** the rendered default shown to the user is "my-tool library"

### Scenario: Plain default without Tera syntax

- **Given** prompt default string contains no `{{` delimiters
- **When** default is resolved
- **Then** the raw string is used as-is without Tera rendering

## Error Cases

| Condition | Behavior |
|-----------|----------|
| User cancels interactive prompt | Returns dialoguer error (propagated via `?`) |
| Tera rendering fails for prompt default | Falls back to raw default string |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `dialoguer` | `Input`, `Select`, `ColorfulTheme` |
| `tera` | `Context`, `Tera` for rendering prompt defaults |
| `chrono` | `Local::now()` for date variables |
| `config` | `Config` for author/org/license defaults |
| `templates` | `Template` struct and `PromptDef` via `template.manifest.prompts` |

### Consumed By

| Module | What is used |
|--------|-------------|
| `init` | `select_template()`, `prompt_variables()` |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-04-21 | Add `author_override` and `org_override` params to `prompt_variables` signature |
| 2 | 2026-04-18 | Fill API descriptions, add license invariant, add prompt ordering invariant, add Tera/plain default scenarios |
| 1 | 2026-04-18 | Initial spec |
