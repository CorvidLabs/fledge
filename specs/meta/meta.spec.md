---
module: meta
version: 1
status: active
files:
  - src/meta.rs

db_tables: []
depends_on: []
---

# Meta

## Purpose

Provides project metadata modeling and content-hash helpers for fledge. After a template is scaffolded, fledge records provenance (`ProjectMeta` / `SourceInfo`) and per-file SHA-256 hashes into `.fledge/meta.toml`, so introspect and spec tooling can identify which template produced a project, at what version, and whether generated files have drifted from their original content.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `ProjectMeta` | Top-level project-provenance record serialized to `.fledge/meta.toml` |
| `SourceInfo` | Provenance for the generating template (name, ref/version, fledge version, dates) |
| `compute_file_hash` | Computes the lowercase hex SHA-256 digest of a byte slice; used for per-file drift detection |
| `write_project_meta` | Serializes project provenance and file hashes to `.fledge/meta.toml`, creating the `.fledge` directory and its `.gitignore` if missing |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ProjectMeta` | Top-level metadata record: `source` provenance, template `variables`, and per-file content `files` hashes; serialized to TOML |
| `SourceInfo` | Provenance for the generating template: template name, optional remote/git ref/version, the `fledge_version` that created it, `created` date, and optional `updated` date |

## Invariants

1. `compute_file_hash` always returns a 64-character lowercase hexadecimal string (SHA-256).
2. `compute_file_hash` is deterministic — identical input bytes always produce identical output.
3. `write_project_meta` only records a hash for created files that exist and are regular files at write time; missing paths are silently skipped.
4. `SourceInfo.fledge_version` is captured from `env!("CARGO_PKG_VERSION")` at compile time, and `created` is the local date in `YYYY-MM-DD` format.
5. `write_project_meta` writes metadata to `.fledge/meta.toml` and ensures a `.fledge/.gitignore` exists (never overwriting an existing one).
6. Only string-valued template variables are persisted into `ProjectMeta.variables`; non-string values are dropped.

## Behavioral Examples

### Scenario: Hashing file content
```
Given the byte slice b"hello world"
When compute_file_hash is called
Then it returns "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
```

### Scenario: Writing project metadata after scaffolding
```
Given a project directory and a list of created files
When write_project_meta is called with the template name and variables
Then .fledge/meta.toml is written with source provenance and a hash for each existing created file
And a .fledge/.gitignore is created if one does not already exist
```

### Scenario: Skipping a non-existent created file
```
Given a created-files list that includes a path which no longer exists on disk
When write_project_meta is called
Then that path is omitted from the files hash map and no error is raised
```

## Error Cases

| Error | Condition |
|-------|-----------|
| creating .fledge directory | The `.fledge` directory cannot be created |
| reading <file> for hash | A created file exists but cannot be read for hashing |
| serializing project metadata | The `ProjectMeta` record fails to serialize to TOML |
| writing .fledge/meta.toml | The metadata file cannot be written |
| writing .fledge/.gitignore | The `.gitignore` file cannot be written |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `sha2` | SHA-256 hashing for `compute_file_hash` |
| `serde` | `Serialize` / `Deserialize` derives for `ProjectMeta` and `SourceInfo` |
| `toml` | Serializing metadata to TOML |
| `tera` | `tera::Context` input for template variables |
| `chrono` | Local date for the `created` field |
| `anyhow` | Error context and `Result` propagation |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-07-03 | Initial spec |
