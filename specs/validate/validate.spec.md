---
module: validate
version: 1
status: active
files:
  - src/validate.rs

db_tables: []
depends_on:
  - templates
---

# Validate

## Purpose

Validates fledge template directories for correctness before publishing or use. Checks manifest parsing, Tera syntax, variable definitions, file coverage, and render glob matching. Supports single template or batch validation of a directory of templates.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `ValidateOptions` | Options struct for the validate-template command |
| `run` | Entry point that dispatches to single or batch validation |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ValidateOptions` | Command options: path, strict mode, JSON output |
| `ValidationReport` | Per-template results: template name, path, errors, warnings |

## Invariants

1. A missing or unparseable `template.toml` is always an error
2. An empty `template.name` or `template.description` is always an error
3. Broken Tera syntax in rendered files is an error
4. Undefined variables (not in builtins or prompts) are warnings
5. Render globs that match no files are warnings
6. `template.toml` not in `files.ignore` is a warning
7. In strict mode, warnings are promoted to errors (non-zero exit)
8. JSON mode outputs an array of `ValidationReport` objects
9. GitHub Actions `${{ }}` expressions are not flagged as Tera variables
10. `.tera` extension files are always validated regardless of render globs

## Behavioral Examples

### Scenario: Valid template passes
```
Given a directory with a valid template.toml and matching files
When the user runs `fledge templates validate ./my-template`
Then a green checkmark and "valid" are shown
And exit code is 0
```

### Scenario: Batch validation
```
Given a directory containing multiple template subdirectories
When the user runs `fledge templates validate ./templates`
Then each template is validated independently
And a summary line shows total templates, errors, and warnings
```

### Scenario: Strict mode
```
Given a template with warnings but no errors
When the user runs `fledge templates validate ./my-template --strict`
Then exit code is non-zero
```

### Scenario: JSON output
```
Given any template directory
When the user runs `fledge templates validate ./my-template --json`
Then output is a JSON array of ValidationReport objects
```

### Scenario: Broken Tera syntax
```
Given a template with a file containing `{{ broken unclosed`
When the user runs `fledge templates validate ./my-template`
Then the file is reported with a Tera syntax error
```

### Scenario: Undefined variable
```
Given a rendered file using `{{ custom_var }}` with no matching prompt
When the user runs `fledge templates validate ./my-template`
Then a warning is shown: "uses undefined variable 'custom_var'"
```

## Error Cases

| Error | Condition |
|-------|-----------|
| Cannot read template.toml | File missing or unreadable |
| Invalid template.toml | TOML parse error |
| template.name is empty | Name field is empty string |
| template.description is empty | Description field is empty string |
| Tera syntax error | Rendered file has invalid Tera syntax |
| No templates found | Batch mode directory has no template subdirectories |
| Not a directory | Path argument is not a directory |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `tera` | `Tera` for syntax validation of rendered files |
| `walkdir` | `WalkDir` for recursive file traversal |
| `regex_lite` | Variable extraction from Tera templates |
| `toml` | Manifest deserialization |
| `console` | `style` for colored output |
| `serde` / `serde_json` | JSON serialization for `--json` mode |
| `anyhow` | Error handling |
| `templates` | `TemplateManifest`, `matches_glob_pub` for glob matching |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
