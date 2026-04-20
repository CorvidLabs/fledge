---
module: metrics
version: 1
status: active
files:
  - src/metrics.rs

db_tables: []
depends_on:
  - run
  - config
---

# Metrics

## Purpose

Project code metrics — lines of code by language, file churn from git history, and test file detection with test-to-code ratio.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — compute and display project metrics |
| `MetricsOptions` | Options: `churn`, `tests`, `json`, `limit` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `MetricsOptions` | Options: `churn`, `tests`, `json`, `limit` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(MetricsOptions) -> Result<()>` | Main entry — dispatch to LOC/churn/tests |

## Invariants

1. Default mode counts lines of code grouped by language, excluding ignored directories
2. `--churn` shows files sorted by commit frequency from git history, filtered to existing files
3. `--tests` detects test files using language-specific patterns and reports test-to-code ratio
4. `--json` outputs structured JSON for all modes
5. Directories like `.git`, `node_modules`, `target`, `vendor`, `dist`, `build` are always excluded
6. Lines are classified as code, blank, or comment based on language-specific comment prefixes
7. `--limit` controls how many entries appear in churn output (default 20)

## Behavioral Examples

```
# Default: LOC by language
$ fledge metrics
fledge metrics

  Project type: rust

  Lines of Code
  Language            Files    Lines     Code    Blank  Comment
  Rust                   20     5000     4200      600      200
  Markdown               10      500      350      150        0
  Total                  30     5500     4550      750      200

# Churn analysis
$ fledge metrics --churn --limit 5
fledge metrics --churn

  Commits  File
  19       src/main.rs
  14       Cargo.toml
  11       src/lib.rs
  6        README.md
  5        src/config.rs

# Test detection
$ fledge metrics --tests
fledge metrics --tests

  Test files: 4
  Source files: 30
  Test ratio: 13.3%

  Test files:
    tests/integration.rs
    src/foo_test.rs
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Not a git repo | `--churn` outside git repo | Error with message |

## Dependencies

- run (uses `detect_project_type`)
- walkdir (file traversal)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
