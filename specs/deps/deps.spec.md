---
module: deps
version: 1
status: active
files:
  - src/deps.rs

db_tables: []
depends_on:
  - run
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
7. Supported ecosystems: Rust (Cargo.lock), Node (package-lock.json, yarn.lock, bun.lock), Go (go.sum), Python (requirements.txt, Pipfile.lock, poetry.lock, uv.lock), Ruby (Gemfile.lock), Swift (Package.resolved v1 and v2+), Java/Kotlin (gradle.lockfile or `gradle dependencies`, Maven via `mvn dependency:list`)
8. Java/Kotlin Gradle uses gradle.lockfile when available, falls back to running `./gradlew dependencies`; Maven runs `mvn dependency:list`
9. Swift and Java/Kotlin license scanning gracefully reports it is not yet supported

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
| 2 | 2026-04-23 | Add Swift/SPM support (Package.resolved v1 and v2+) |
| 3 | 2026-04-23 | Add Java/Kotlin Gradle support (gradle.lockfile + gradlew fallback) and Maven support |
