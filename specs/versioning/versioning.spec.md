---
module: versioning
version: 1
status: active
files:
  - src/versioning.rs

db_tables: []
depends_on: []
---

# Versioning

## Purpose

Provides semver version comparison utilities for fledge. Used to enforce `min_fledge_version` constraints from template manifests and to compare template versions.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `Version` | Parsed semver version (major, minor, patch) |
| `check_fledge_version` | Compares required min version against current fledge version, returns error if incompatible |
| `parse_version` | Parses a semver string like "1.2.3" into a Version struct |

### Structs & Enums

| Type | Description |
|------|-------------|
| `Version` | Semver version with major, minor, patch fields, implements `Ord` |

## Invariants

1. Version strings must be in `MAJOR.MINOR.PATCH` format (optional leading `v`)
2. `check_fledge_version` returns `Ok(())` when the constraint is satisfied
3. The current fledge version is read from `env!("CARGO_PKG_VERSION")` at compile time

## Behavioral Examples

### Scenario: Compatible version
```
Given min_fledge_version = "0.2.0"
And the current fledge version is "0.2.1"
Then check_fledge_version returns Ok
```

### Scenario: Incompatible version
```
Given min_fledge_version = "1.0.0"
And the current fledge version is "0.2.1"
Then check_fledge_version returns an error with upgrade instructions
```

### Scenario: No version constraint
```
Given min_fledge_version is None
Then no version check is performed
```

## Error Cases

| Error | Condition |
|-------|-----------|
| Invalid version string | Version string doesn't match semver format |
| Incompatible version | Current fledge version is older than required |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| (none) | Pure Rust, no external dependencies |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec |
