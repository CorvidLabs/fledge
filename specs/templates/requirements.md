---
spec: templates.spec.md
---

## User Stories

- As a user, I want built-in templates available immediately after `cargo install fledge`
- As a user, I want to add my own template directories and have them discovered alongside built-ins
- As a user, I want templates from remote GitHub repos to appear in my template list
- As a template author, I want to define which files get Tera rendering via glob patterns in `template.toml`
- As a template author, I want `.tera` files to always be rendered with the extension stripped
- As a template author, I want to use template variables in file and directory names

## Durable Requirements

### REQ-templates-001

The implementation SHALL satisfy the following criterion: `discover_templates()` finds all built-in templates (8 language starters: `go-cli`, `kotlin-kmp`, `kotlin-ktor-api`, `python-cli`, `rust-cli`, `static-site`, `ts-bun`, `ts-node`; plus setup-only `fledge-plugin` and `corvid-stack`)

Acceptance Criteria

- `discover_templates()` finds all built-in templates (8 language starters: `go-cli`, `kotlin-kmp`, `kotlin-ktor-api`, `python-cli`, `rust-cli`, `static-site`, `ts-bun`, `ts-node`; plus setup-only `fledge-plugin` and `corvid-stack`)

### REQ-templates-002

The implementation SHALL satisfy the following criterion: Extra paths from config are searched for template directories

Acceptance Criteria

- Extra paths from config are searched for template directories

### REQ-templates-003

The implementation SHALL satisfy the following criterion: Remote repos from config are fetched and searched for templates

Acceptance Criteria

- Remote repos from config are fetched and searched for templates

### REQ-templates-004

The implementation SHALL satisfy the following criterion: Templates are returned sorted alphabetically by name

Acceptance Criteria

- Templates are returned sorted alphabetically by name

### REQ-templates-005

The implementation SHALL satisfy the following criterion: Directories without `template.toml` are silently skipped

Acceptance Criteria

- Directories without `template.toml` are silently skipped

### REQ-templates-006

The implementation SHALL satisfy the following criterion: Non-existent extra paths are silently skipped

Acceptance Criteria

- Non-existent extra paths are silently skipped

### REQ-templates-007

The implementation SHALL satisfy the following criterion: `render_template()` renders `.tera` files and strips the extension

Acceptance Criteria

- `render_template()` renders `.tera` files and strips the extension

### REQ-templates-008

The implementation SHALL satisfy the following criterion: `render_template()` renders files matching `files.render` globs

Acceptance Criteria

- `render_template()` renders files matching `files.render` globs

### REQ-templates-009

The implementation SHALL satisfy the following criterion: `render_template()` copies non-matching files as-is

Acceptance Criteria

- `render_template()` copies non-matching files as-is

### REQ-templates-010

The implementation SHALL satisfy the following criterion: `render_template()` skips files matching `files.ignore` globs

Acceptance Criteria

- `render_template()` skips files matching `files.ignore` globs

### REQ-templates-011

The implementation SHALL satisfy the following criterion: `render_template()` renders Tera variables in file/directory paths

Acceptance Criteria

- `render_template()` renders Tera variables in file/directory paths

### REQ-templates-012

The implementation SHALL satisfy the following criterion: Created files list is returned sorted alphabetically

Acceptance Criteria

- Created files list is returned sorted alphabetically

## Acceptance Criteria

- `discover_templates()` finds all built-in templates (8 language starters: `go-cli`, `kotlin-kmp`, `kotlin-ktor-api`, `python-cli`, `rust-cli`, `static-site`, `ts-bun`, `ts-node`; plus setup-only `fledge-plugin` and `corvid-stack`)
- Extra paths from config are searched for template directories
- Remote repos from config are fetched and searched for templates
- Templates are returned sorted alphabetically by name
- Directories without `template.toml` are silently skipped
- Non-existent extra paths are silently skipped
- `render_template()` renders `.tera` files and strips the extension
- `render_template()` renders files matching `files.render` globs
- `render_template()` copies non-matching files as-is
- `render_template()` skips files matching `files.ignore` globs
- `render_template()` renders Tera variables in file/directory paths
- Created files list is returned sorted alphabetically

## Constraints

- Embedded templates use `include_dir!` — binary size includes all template files
- Template extraction uses a version-stamped cache to avoid stale templates after upgrades
- Glob matching must handle `*` (single segment), `**` (any depth), and dot escaping

## Out of Scope

- Template versioning or compatibility checking
- Template creation, publishing, or remote search (handled by sibling `templates` subcommands `create`, `publish`, `search` — see their respective handlers in `main.rs` and the `create_template`, `publish`, and `search` library modules)
- Template inheritance or composition
