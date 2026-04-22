---
module: templates
version: 4
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
| `load_templates_from_dir_pub` | Loads templates from a specific directory into a mutable vector |
| `matches_glob_pub` | Tests whether a file path matches a glob pattern |
| `check_requirements` | Checks which required tools from `template.toml` are available on PATH |

### Structs & Enums

| Type | Description |
|------|-------------|
| `Template` | A discovered template with name, description, path, and manifest |
| `TemplateManifest` | Parsed `template.toml` with info, prompts, and file rules |
| `TemplateInfo` | Template name, description, version, min_fledge_version, and requires (tool dependencies) |
| `PromptDef` | Custom prompt definition with message and optional default |
| `FileRules` | Glob patterns for render, copy, and ignore |
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
| `load_templates_from_dir_pub` | `(&Path, &mut Vec<Template>) -> Result<()>` | Load templates from a directory into a vector |
| `matches_glob_pub` | `(&str, &str) -> bool` | Test if a path matches a glob pattern |
| `check_requirements` | `(&[String]) -> (Vec<String>, Vec<String>)` | Returns (found, missing) tools from PATH |

## Invariants

1. Templates are sorted alphabetically by name after discovery
2. Files ending in `.tera` are always rendered and the extension is stripped
3. Files matching `render` globs are rendered through Tera
4. Files matching `ignore` globs are skipped entirely
5. Tera expressions in file paths (e.g., `{{ project_name_pascal }}`) are resolved
6. Parent directories are created automatically during rendering
7. The list of created files returned by `render_template` is sorted alphabetically
8. Directories without a `template.toml` are silently skipped during discovery
9. Built-in template directory resolution checks exe-relative, then `CARGO_MANIFEST_DIR`, then falls back to `./templates`

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

| Date | Author | Change |
|------|--------|--------|
| 2026-04-18 | CorvidAgent | Initial spec |
| 2026-04-18 | CorvidAgent | v2: Fill in export descriptions, add invariants for sort order and directory resolution, expand behavioral examples and error cases |
| 2026-04-18 | CorvidAgent | v3: Add discover_templates_with_repos for remote GitHub template support |
| 2026-04-20 | CorvidAgent | v4: Add check_requirements for template tool dependency checking |
