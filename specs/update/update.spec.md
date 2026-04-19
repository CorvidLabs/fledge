---
module: update
version: 1
status: active
files:
  - src/update.rs

db_tables: []
depends_on:
  - specs/init/init.spec.md
  - specs/templates/templates.spec.md
  - specs/remote/remote.spec.md
---

# Update

## Purpose

Re-applies a template to an existing project that was scaffolded with `fledge init`. Reads `.fledge.toml` (written by init) to determine the source template and original variables, fetches the latest template version, and applies changes — automatically for unmodified files, skipping files the user has changed.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `UpdateOptions` | Configuration struct for update command passed from CLI |
| `ProjectMeta` | Deserialized `.fledge.toml` — source template, variables, file hashes |
| `SourceInfo` | Template source: name, remote ref, git ref, fledge version |
| `UpdateAction` | Enum: Add, Update, Skip (user-modified), Remove (template-deleted) |
| `run` | Main entry point that drives the update workflow |
| `compute_file_hash` | SHA-256 hash of file contents for change detection |
| `write_project_meta` | Writes `.fledge.toml` with template source info, variables, and file hashes |

### Structs & Enums

| Type | Description |
|------|-------------|
| `UpdateOptions` | Options: dry_run, refresh, yes |
| `ProjectMeta` | Deserialized `.fledge.toml` — source template, variables, file hashes |
| `SourceInfo` | Template source: name, remote ref, git ref, fledge version |
| `UpdateAction` | Enum: Add, Update, Skip (user-modified), Remove (template-deleted) |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(UpdateOptions) -> Result<()>` | Main entry point for `fledge update` |
| `compute_file_hash` | `(&[u8]) -> String` | SHA-256 hash of file contents |
| `write_project_meta` | `(&Path, &str, Option<&str>, Option<&str>, Option<&str>, &Context, &[PathBuf]) -> Result<()>` | Writes `.fledge.toml` with template metadata and file hashes |

## Invariants

1. `.fledge.toml` must exist in the project root — bails if missing
2. User-modified files are never overwritten without explicit confirmation
3. New files from the template are always added
4. Deleted template files produce a warning but are not removed
5. `.fledge.toml` is updated after a successful update with new hashes and version

## Behavioral Examples

### Scenario: Dry run

- **Given** a project with `.fledge.toml` pointing to `rust-cli`
- **When** `fledge update --dry-run` is run
- **Then** shows list of files that would be added, updated, or skipped — writes nothing

### Scenario: Unmodified file updated in template

- **Given** `.github/workflows/ci.yml` hash matches the original
- **When** template has a newer version of that file
- **Then** file is overwritten with the new version

### Scenario: User-modified file

- **Given** `README.md` hash does NOT match the original
- **When** template has a newer version
- **Then** file is skipped with a warning

### Scenario: New file in template

- **Given** template now includes `CONTRIBUTING.md` that didn't exist before
- **When** `fledge update` runs
- **Then** `CONTRIBUTING.md` is added to the project

### Scenario: File removed from template

- **Given** `old-config.yml` was in the original template but is now gone
- **When** `fledge update` runs
- **Then** warning is printed but file is NOT deleted

### Scenario: No .fledge.toml

- **Given** project was not created with fledge or has no `.fledge.toml`
- **When** `fledge update` is run
- **Then** errors with "No .fledge.toml found"

### Scenario: Remote template

- **Given** `.fledge.toml` has `remote = "CorvidLabs/templates/rust-cli"`
- **When** `fledge update` runs
- **Then** fetches latest from GitHub, diffs, and applies

## Error Cases

| Condition | Behavior |
|-----------|----------|
| No `.fledge.toml` | Bails with "No .fledge.toml found. Was this project created with fledge?" |
| Invalid `.fledge.toml` | Bails with parse error |
| Template not found | Bails with template name and suggestion |
| Remote fetch fails | Bails with network error context |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `config` | `Config::load()`, `extra_template_paths()`, `github_token()` |
| `templates` | `discover_templates_with_repos()`, `render_template()`, `TemplateManifest` |
| `remote` | `is_remote_ref()`, `parse_remote_ref()`, `resolve_template_dir()` |
| `console` | `style()` for colored output |
| `anyhow` | Error handling |
| `toml` | Parsing `.fledge.toml` |
| `serde` | Serialization/deserialization |

### Consumed By

| Module | What is used |
|--------|-------------|
| `main` | `run()` called from `Commands::Update` |

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-04-19 | CorvidAgent | Initial spec |
