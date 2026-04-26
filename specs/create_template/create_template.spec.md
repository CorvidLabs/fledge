---
module: create_template
version: 2
status: active
files:
  - src/create_template.rs

db_tables: []
depends_on:
  - templates
---

# Create Template

## Purpose

Scaffolds a new fledge template project with a valid `template.toml` manifest, example files, and author documentation. Makes it trivial for anyone to create and share templates.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `CreateTemplateOptions` | Options struct for the create-template command (name, output directory) |
| `run` | Entry point that checks for existing directory, gathers interactive answers, and scaffolds the template |

### Structs & Enums

| Type | Description |
|------|-------------|
| `CreateTemplateOptions` | Command options: template name and output parent directory |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(CreateTemplateOptions) -> Result<()>` | Validate target doesn't exist, prompt user, scaffold template directory |

## Invariants

1. Fails immediately if target directory already exists
2. Generated `template.toml` is always valid TOML parseable as `TemplateManifest`
3. `template.toml` always includes `[template]` and `[files]` sections
4. `template.toml` always ignores itself (`template.toml` in ignore list)
5. Interactive prompts provide sensible defaults for all fields
6. Hooks section is only included when user opts in
7. Custom prompts section is only included when user opts in
8. Scaffolded template includes example `.tera` file demonstrating variable substitution
9. Scaffolded template includes author-facing README with testing instructions
10. `--json` emits a single `{schema_version: 1, action: "create", path, name, description, render_patterns, include_hooks, include_prompts, files_created}` envelope on stdout. Prose suppressed; JSON mode implies non-interactive (`yes = true`)

## Behavioral Examples

### Scenario: Basic template creation

- **Given** directory `my-template` does not exist in the output directory
- **When** `run(CreateTemplateOptions { name: "my-template", output: "." })` is called and user accepts defaults
- **Then** creates `my-template/` with `template.toml`, `README.md`, `README.md.tera`, `.gitignore`

### Scenario: Target directory already exists

- **Given** directory `my-template` already exists
- **When** `run(CreateTemplateOptions { name: "my-template", output: "." })` is called
- **Then** returns error "Directory 'my-template' already exists"

### Scenario: User opts into hooks

- **Given** user confirms "Include post-create hooks?"
- **When** template is scaffolded
- **Then** `template.toml` contains a `[hooks]` section with commented examples

### Scenario: User opts out of custom prompts

- **Given** user declines "Include custom prompts?"
- **When** template is scaffolded
- **Then** `template.toml` has no `[prompts]` section

## Error Cases

| Condition | Behavior |
|-----------|----------|
| Target directory already exists | Returns error with directory path |
| Cannot create target directory | Returns IO error with context |
| Interactive prompt fails (e.g., non-TTY) | Returns dialoguer error |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `dialoguer` | `Input`, `Confirm` for interactive prompts |
| `console` | `style` for colored output |
| `anyhow` | Error handling |

### Consumed By

| Module | What is used |
|--------|-------------|
| `main` | `run()`, `CreateTemplateOptions` for the `create-template` subcommand |

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-04-19 | CorvidAgent | Initial spec |
| 2026-04-25 | 0xLeif | v2: `--json` emits structured envelope (schema_version: 1) for `templates create`; prose suppressed, JSON mode implies non-interactive |
