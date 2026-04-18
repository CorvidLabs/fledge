---
module: init
version: 3
status: active
files:
  - src/init.rs

db_tables: []
depends_on:
  - templates
---

# Init

## Purpose

Orchestrates project creation from a template. Resolves the template, prompts for variables, creates the project directory, renders files, optionally initializes git, and prints a summary.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `InitOptions` | Configuration struct for project creation passed from CLI |
| `run` | Main entry point that drives the full init workflow |
| `init_git_for_tui` | TUI-only wrapper around `init_git` (feature-gated behind `tui`) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `InitOptions` | Options for project creation: name, template, output, no_git, no_install |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(InitOptions) -> Result<()>` | Main entry point for `fledge init` |

## Invariants

1. Target directory must not already exist — bails if it does
2. At least one template must be available
3. Git init creates an initial commit with all scaffolded files
4. Directory is created before template rendering begins

## Behavioral Examples

### Scenario: Template specified via flag

- **Given** `--template rust-cli` is passed
- **When** `run()` is called
- **Then** uses `rust-cli` without prompting, renders files, inits git

### Scenario: Remote template via owner/repo

- **Given** `--template CorvidLabs/fledge-templates/rust-cli` is passed
- **When** `run()` is called
- **Then** clones the GitHub repo, finds the template, renders files, inits git

### Scenario: Directory already exists

- **Given** target directory `./my-project` already exists
- **When** `run()` is called with name "my-project"
- **Then** returns error with message to choose a different name

### Scenario: No git

- **Given** `--no-git` flag is set
- **When** `run()` completes
- **Then** project directory has no `.git` folder

## Error Cases

| Condition | Behavior |
|-----------|----------|
| No templates found | Bails with "No templates found" |
| Template name not found | Bails with available template names listed |
| Target directory exists | Bails with exit code 3 |
| Git init fails | Bails with "git init failed" |
| Template rendering fails | Propagates Tera error |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `config` | `Config::load()`, `extra_template_paths()`, `github_token()`, `template_repos()` |
| `templates` | `discover_templates_with_repos()`, `render_template()` |
| `remote` | `is_remote_ref()`, `parse_remote_ref()`, `resolve_template_dir()` |
| `prompts` | `select_template()`, `prompt_variables()` |
| `console` | `style()` for colored output |
| `anyhow` | Error handling |

### Consumed By

| Module | What is used |
|--------|-------------|
| `main` | `run()` called from `Commands::Init` |

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-04-18 | CorvidAgent | Initial spec |
| 2026-04-18 | CorvidAgent | v2: fill in export descriptions, re-validate against source |
| 2026-04-18 | CorvidAgent | v3: add remote template support via owner/repo syntax |
