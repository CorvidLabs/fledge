---
module: update
version: 1
status: active
files:
  - src/update.rs

db_tables: []
depends_on:
  - init
  - templates
  - remote
---

# Update

## Purpose

Re-applies a template to an existing project that was scaffolded with `fledge init`. Reads `.fledge/meta.toml` (written by init) to determine the source template and original variables, fetches the latest template version, and applies changes — automatically for unmodified files, skipping files the user has changed. Supports legacy `.fledge.toml` location for backwards compatibility.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `UpdateOptions` | Configuration struct for update command passed from CLI |
| `ProjectMeta` | Deserialized `.fledge/meta.toml` — source template, variables, file hashes |
| `SourceInfo` | Template source: name, remote ref, git ref, fledge version |
| `UpdateAction` | Enum: Add, Update, Skip (user-modified), Remove (template-deleted) |
| `run` | Main entry point that drives the update workflow |
| `compute_file_hash` | SHA-256 hash of file contents for change detection |
| `write_project_meta` | Writes `.fledge/meta.toml` with template source info, variables, and file hashes |
| `resolve_meta_path` | Resolves project metadata path (`.fledge/meta.toml` or legacy `.fledge.toml`) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `UpdateOptions` | Options: dry_run, refresh |
| `ProjectMeta` | Deserialized `.fledge/meta.toml` — source template, variables, file hashes |
| `SourceInfo` | Template source: name, remote ref, git ref, fledge version |
| `UpdateAction` | Enum: Add, Update, Skip (user-modified), Remove (template-deleted) |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(UpdateOptions) -> Result<()>` | Main entry point for `fledge update` |
| `compute_file_hash` | `(&[u8]) -> String` | SHA-256 hash of file contents |
| `write_project_meta` | `(&Path, &str, Option<&str>, Option<&str>, Option<&str>, &Context, &[PathBuf]) -> Result<()>` | Writes `.fledge/meta.toml` with template metadata and file hashes |
| `resolve_meta_path` | `(&Path) -> Option<PathBuf>` | Resolves metadata path — prefers `.fledge/meta.toml`, falls back to `.fledge.toml` |

## Invariants

1. Project metadata must exist (`.fledge/meta.toml` or legacy `.fledge.toml`) — bails if missing
2. User-modified files are never overwritten without explicit confirmation
3. New files from the template are always added
4. Deleted template files produce a warning but are not removed
5. `.fledge/meta.toml` is updated after a successful update with new hashes and version
6. On update, legacy `.fledge.toml` is migrated to `.fledge/meta.toml` and the old file is removed
7. `write_project_meta` creates `.fledge/` directory and `.fledge/.gitignore` if missing

## Behavioral Examples

### Scenario: Dry run

- **Given** a project with `.fledge/meta.toml` pointing to `rust-cli`
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

### Scenario: No project metadata

- **Given** project was not created with fledge or has no `.fledge/meta.toml`
- **When** `fledge update` is run
- **Then** errors with "No .fledge/meta.toml found"

### Scenario: Legacy .fledge.toml migration

- **Given** project has `.fledge.toml` but no `.fledge/meta.toml`
- **When** `fledge update` runs
- **Then** reads from legacy location, writes updated metadata to `.fledge/meta.toml`, and removes old `.fledge.toml`

### Scenario: Remote template

- **Given** `.fledge/meta.toml` has `remote = "CorvidLabs/templates/rust-cli"`
- **When** `fledge update` runs
- **Then** fetches latest from GitHub, diffs, and applies

## Error Cases

| Condition | Behavior |
|-----------|----------|
| No `.fledge/meta.toml` or `.fledge.toml` | Bails with "No .fledge/meta.toml found. Was this project created with fledge?" |
| Invalid project metadata | Bails with parse error |
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
| `toml` | Parsing `.fledge/meta.toml` |
| `serde` | Serialization/deserialization |

### Consumed By

| Module | What is used |
|--------|-------------|
| `main` | `run()` called from `Commands::Update` |

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-04-21 | CorvidAgent | Move `.fledge.toml` to `.fledge/meta.toml` with backwards compat; add `.fledge/.gitignore` |
| 2026-04-19 | CorvidAgent | Initial spec |
