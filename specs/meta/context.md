---
spec: meta.spec.md
---

# Meta — Context

## Problem

After a template scaffolds a project, nothing records where the project came from or what its generated files originally contained. Without provenance, introspect and spec tooling can't tell which template (and version) produced a project, and drift from the original generated content can't be detected.

## Solution

A small metadata module that models project provenance (`ProjectMeta` / `SourceInfo`) and writes it, along with per-file SHA-256 hashes, to `.fledge/meta.toml` after scaffolding. A companion `.fledge/.gitignore` keeps the local cache out of version control.

## Key Decisions

- Hashes are lowercase hex SHA-256 via `sha2`, computed over raw file bytes for deterministic drift detection
- Only string-valued template variables are persisted; non-string values are dropped to keep the record flat and TOML-friendly
- Missing created files are skipped rather than erroring, since scaffolding may legitimately leave some paths absent
- `.fledge/.gitignore` is written only when absent, never overwriting user edits

## Files to Read First

- `src/meta.rs` — the entire module (structs, `compute_file_hash`, `write_project_meta`)

## Current Status

Active. Used after template scaffolding to record provenance and file hashes.

## Notes

- `fledge_version` comes from `env!("CARGO_PKG_VERSION")` at compile time
- `created` is the local date (`YYYY-MM-DD`); `updated` is always written as `None`
