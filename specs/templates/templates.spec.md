---
module: templates
version: 8
status: active
files:
  - src/templates.rs

db_tables: []
depends_on: []
---

# Templates

## Purpose

Template discovery, loading, and rendering. Finds templates from built-in and user-configured directories, parses `template.toml` manifests, and renders project files through Tera with variable substitution.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `TemplateManifest` | Parsed representation of a `template.toml` manifest file |
| `TemplateInfo` | Metadata about a template: name, description, and optional minimum version |
| `PromptDef` | Definition for a user-facing prompt with message text and optional default value |
| `FileRules` | Glob patterns controlling which files are rendered, copied, or ignored |
| `Hooks` | Post-create hook commands defined in `[hooks]` section of template.toml |
| `Template` | A discovered template combining its name, description, directory path, and parsed manifest |
| `discover_templates` | Scans built-in and extra directories for valid templates |
| `discover_templates_with_repos` | Discovers templates from local paths and remote GitHub repos |
| `render_template` | Renders a template's files into a target directory using Tera variable substitution |
| `matches_glob_pub` | Tests whether a file path matches a glob pattern |
| `check_requirements` | Checks which required tools from `template.toml` are available on PATH |
| `TEMPLATES_LIST_SCHEMA` | Per-command JSON schema version for `templates list --json` envelope |
| `TEMPLATES_SEARCH_SCHEMA` | Per-command JSON schema version for `templates search --json` envelope |
| `TEMPLATES_PUBLISH_SCHEMA` | Per-command JSON schema version for `templates publish --json` envelope |

### Structs & Enums

| Type | Description |
|------|-------------|
| `Template` | A discovered template with name, description, path, and manifest |
| `TemplateManifest` | Parsed `template.toml` with info, prompts, and file rules |
| `TemplateInfo` | Template name, description, version, min_fledge_version, and requires (tool dependencies) |
| `PromptDef` | Custom prompt definition with message and optional default. Stored in a `BTreeMap<String, PromptDef>` so iteration order is deterministic (alphabetical by key) |
| `FileRules` | Glob patterns for render, copy, and ignore. Precedence: `ignore` short-circuits → `.tera` extension always renders → `copy` forces verbatim → `render` Tera-renders → default copies |
| `Hooks` | Post-create lifecycle hooks (e.g., `npm install`, `bun install`) |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `discover_templates` | `(&[PathBuf]) -> Result<Vec<Template>>` | Find all templates from built-in and extra paths |
| `discover_templates_with_repos` | `(&[PathBuf], &[String], Option<&str>) -> Result<Vec<Template>>` | Find templates from local paths and remote GitHub repos |
| `render_template` | `(&Template, &Path, &tera::Context) -> Result<Vec<PathBuf>>` | Render template files into target directory |
| `matches_glob_pub` | `(&str, &str) -> bool` | Test if a path matches a glob pattern |
| `check_requirements` | `(&[String]) -> (Vec<String>, Vec<String>)` | Returns (found, missing) tools from PATH |

## Invariants

1. Templates are sorted alphabetically by name after discovery
2. Files ending in `.tera` are always rendered and the extension is stripped — this is the explicit "render this" signal and overrides `copy`
3. Files matching `copy` globs are copied verbatim (never run through Tera) even when a `render` glob would otherwise match them. The `.tera` extension still wins
4. Files matching `render` globs (and not `copy`) are rendered through Tera
5. Files matching `ignore` globs are skipped entirely (highest precedence after errors)
6. Files that match no glob are copied as bytes (default)
7. Tera expressions in file paths (e.g., `{{ project_name_pascal }}`) are resolved
8. Parent directories are created automatically during rendering
9. The list of created files returned by `render_template` is sorted alphabetically
10. Directories without a `template.toml` are silently skipped during discovery
11. Built-in template directory resolution checks exe-relative, then `CARGO_MANIFEST_DIR`, then falls back to `./templates`
12. `prompts` are iterated in alphabetical order by key — multi-prompt templates ask questions in a stable order across runs

## Behavioral Examples

### Scenario: Built-in template discovery

- **Given** `templates/rust-cli/template.toml` exists
- **When** `discover_templates(&[])` is called
- **Then** returns a `Template` with name "rust-cli" and its manifest

### Scenario: Extra template directory

- **Given** an extra path `/home/user/.fledge/templates/` contains `my-template/template.toml`
- **When** `discover_templates(&[PathBuf::from("/home/user/.fledge/templates/")])` is called
- **Then** returns templates from both built-in and the extra directory, sorted by name

### Scenario: Non-existent extra path is ignored

- **Given** an extra path `/nonexistent/` does not exist
- **When** `discover_templates(&[PathBuf::from("/nonexistent/")])` is called
- **Then** silently skips it and returns only built-in templates

### Scenario: Tera file rendering

- **Given** template contains `Cargo.toml.tera` with `{{ project_name }}`
- **When** `render_template()` is called with project_name="my-app"
- **Then** creates `Cargo.toml` (no .tera extension) with "my-app" substituted

### Scenario: Render-glob file rendering

- **Given** template contains `README.md` and `files.render` includes `README.md`
- **When** `render_template()` is called
- **Then** `README.md` is processed through Tera before writing

### Scenario: Non-rendered file

- **Given** template contains `.gitignore` not in render globs and without `.tera` extension
- **When** `render_template()` is called
- **Then** file is copied as-is without Tera processing

### Scenario: Path variable substitution

- **Given** template contains a directory named `{{ project_name_pascal }}`
- **When** `render_template()` is called with project_name_pascal="MyApp"
- **Then** the output directory is named `MyApp`

## Error Cases

| Condition | Behavior |
|-----------|----------|
| `template.toml` is malformed | Returns TOML parse error with file path context |
| Tera syntax error in template file | Returns error with template filename |
| Template variable missing | Returns Tera rendering error |
| Template directory not readable | Returns IO error |
| Tera expression in file path is invalid | Returns Tera rendering error |

## Compatibility Policy

`templates v1` is the stable manifest contract that ships with fledge 1.0. To
protect template authors from breakage, the following rules govern how
`template.toml` and the `Template`/`TemplateManifest` Rust API may evolve within
the v1 major version:

1. **Additive-only sections.** New top-level sections (`[plugins]`, `[ci]`, `[i18n]`, …) may be added at any time. Templates and fledge already tolerate unknown sections — serde's default deserializer ignores unknown fields, so older fledge ignores newer manifests' extra sections.
2. **No field removal from existing sections.** Once shipped, every field on `[template]`, `[prompts.*]`, `[files]`, and `[hooks]` must continue to be parsed. Removing a field is a breaking change and requires a new manifest version.
3. **No field retyping.** A field's TOML type (string, number, bool, table, array) is locked once shipped. Widening a string into a table, or a single value into an array, is a breaking change.
4. **New optional fields are allowed.** Both fledge and template authors must tolerate unknown fields on known sections — additive optional fields do not require a version bump.
5. **`files` precedence is locked.** `ignore` → `.tera` extension → `copy` → `render` → default-copy. Future glob categories (e.g. `binary`, `executable`) must slot into this precedence without changing the semantics of the existing four.
6. **Prompt iteration order is locked.** Prompts iterate alphabetically by key. Templates that need a specific UX order should name their keys to sort that way (`01_name`, `02_description`).
7. **`hooks` is additive.** New hook stages (`pre_create`, `pre_render`, `post_install`, …) may be added; existing `post_create` semantics must not change. New hooks default to a no-op when absent.
8. **`min_fledge_version` is enforcement-only.** Once a template declares a minimum, fledge will refuse to render with an older version — this contract cannot relax.

Any change that cannot be expressed under these rules requires a new manifest
version declared explicitly (e.g. `[template] manifest_version = 2`); v1
templates continue to render against v1 semantics indefinitely.

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `tera` | `Tera`, `Context` for template rendering |
| `walkdir` | `WalkDir` for recursive directory traversal |
| `toml` | Manifest parsing |
| `serde` | Deserialize for manifest structs |
| `regex_lite` | Simple glob-to-regex matching |

### Consumed By

| Module | What is used |
|--------|-------------|
| `init` | `discover_templates()`, `render_template()` |
| `main` | `discover_templates_with_repos()` for `fledge templates list` |
| `prompts` | `Template`, `PromptDef` types |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 8 | 2026-05-01 | **1.0 contract finalize:** (a) Implement `[files] copy` glob — was documented and present in built-in templates but silently dropped. New precedence: `ignore` → `.tera` → `copy` → `render` → default-copy. Copy-matched files bypass Tera even when a render glob would otherwise catch them. (b) Switch `prompts` from `HashMap` to `BTreeMap` so multi-prompt templates ask questions in a stable alphabetical order across runs. (c) Add Compatibility Policy locking the templates v1 contract (additive-only sections, no field removal/retyping, locked precedence + iteration order) |
| 7 | 2026-04-29 | Add `TEMPLATES_LIST_SCHEMA`, `TEMPLATES_SEARCH_SCHEMA`, `TEMPLATES_PUBLISH_SCHEMA` per-command schema version constants (crate-visible, used by `main.rs` for `--json` envelopes) |
| 6 | 2026-04-26 | **Breaking (1.0 contract finalize):** `templates publish --json` cancelled and success paths now share the same key set (`schema_version`, `action`, `cancelled`, `repo`, `template`, `topic`, `use_hint`). `cancelled` is `true` when user declines, `false` on success. The cancelled `repo.exists` field is removed (`created: false` covers it). Consumers can now read the same keys regardless of cancel/success |
| 5 | 2026-04-25 | Remove `load_templates_from_dir_pub` (was only used by deleted `templates update` and `templates publish`); now an internal `fn` |
| 4 | 2026-04-20 | Add `check_requirements` for template tool dependency checking |
| 3 | 2026-04-18 | Add `discover_templates_with_repos` for remote GitHub template support |
| 2 | 2026-04-18 | Fill in export descriptions, add invariants for sort order and directory resolution, expand behavioral examples and error cases |
| 1 | 2026-04-18 | Initial spec |
