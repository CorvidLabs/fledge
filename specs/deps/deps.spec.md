---
module: deps
version: 1
status: active
files:
  - src/deps.rs

db_tables: []
depends_on:
  - specs/run
---

# Deps

## Purpose

Cross-language dependency health checker. Parses lock files to list dependencies, shells out to ecosystem tools for outdated checks, security audits, and license scanning.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — list deps, check outdated, audit, or scan licenses |
| `DepsOptions` | Options: `outdated`, `audit`, `licenses`, `json` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `DepsOptions` | CLI options: `outdated`, `audit`, `licenses`, `json` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(DepsOptions) -> Result<()>` | Main entry — dispatch to list/outdated/audit/licenses |

## Invariants

1. Project type is detected via `run::detect_project_type`
2. Lock file parsing extracts name + version pairs without network access
3. `--outdated`, `--audit`, and `--licenses` shell out to ecosystem-specific tools
4. Missing ecosystem tools produce a clear error with install guidance
5. Dependencies are always sorted alphabetically by name
6. `--json` outputs a structured `DepsReport` with ecosystem, source, and dependencies array
7. Supported ecosystems: Rust (Cargo.lock), Node (package-lock.json, yarn.lock), Go (go.sum), Python (requirements.txt, Pipfile.lock, poetry.lock), Ruby (Gemfile.lock)
8. Java (Gradle/Maven) gracefully reports that lock file parsing is unsupported

## Behavioral Examples

```
# List dependencies
$ fledge deps
Dependencies (rust via Cargo.lock)
  Total: 42

  Name            Version
  serde           1.0.200
  clap            4.5.0

# JSON output
$ fledge deps --json
{ "ecosystem": "rust", "source": "Cargo.lock", "dependencies": [...] }

# Check outdated
$ fledge deps --outdated
▸ Running outdated check (cargo outdated)...

# Security audit
$ fledge deps --audit
▸ Running security audit (cargo audit)...

# License scan
$ fledge deps --licenses
▸ Running license scan (cargo license)...
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Generic project | No recognized manifest | Error listing supported types |
| No lock file | Lock file missing | Error with install command guidance |
| Tool not found | Ecosystem tool missing | Error with install suggestion |
| Unsupported lock | pnpm-lock.yaml | Error suggesting direct pnpm commands |

## Dependencies

- `run::detect_project_type` for ecosystem detection
- `serde_json` for JSON lock file parsing
- `console` for styled output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
