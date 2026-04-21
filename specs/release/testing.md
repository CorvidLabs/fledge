# Release — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `apply_bump_major` | 1.2.3 → 2.0.0 |
| `apply_bump_minor` | 1.2.3 → 1.3.0 |
| `apply_bump_patch` | 1.2.3 → 1.2.4 |
| `apply_bump_from_zero` | 0.0.0 → all bump levels |
| `extract_toml_version_basic` | Finds version in TOML |
| `extract_toml_version_not_found` | Returns None when missing |
| `detect_version_files_*` | Detects correct files per language |
| `classify_conventional_commits` | Maps feat/fix/docs/etc to labels |
| `strip_prefix_simple` | Removes conventional commit prefixes |
| `read_*_version_test` | Reads version from each file format |
| `bump_cargo_toml` | Updates version in Cargo.toml |
| `bump_package_json` | Updates version in package.json |
| `bump_pyproject_toml` | Updates version in pyproject.toml |
| `preflight_checks_*` | Clean/dirty/not-git/allow-dirty |
| `resolve_explicit_version` | Parses "2.0.0" directly |
| `resolve_bump_from_cargo` | Reads current + bumps |
| `dry_run_no_changes` | No files modified in dry run |
| `full_release_flow` | End-to-end: bump → changelog → commit → tag |
| `release_tag_only_project` | Go project gets tag without file changes |
| `changelog_entry_format` | Sections, links, dates correct |
| `changelog_appends_to_existing` | New entry before old entries |
| `custom_release_files` | [release] files config works |
