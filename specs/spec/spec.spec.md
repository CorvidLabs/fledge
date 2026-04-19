---
module: spec
version: 1
status: active
files:
  - src/spec.rs

db_tables: []
depends_on: []
---

# Spec

## Purpose

Integrates spec-sync validation into fledge as native subcommands. Provides `fledge spec check` to validate specs against source code, `fledge spec init` to scaffold a `.specsync/` configuration directory, and `fledge spec new <name>` to create a new spec module with companion files.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate spec subcommand |
| `SpecAction` | Enum of subcommands: Check, Init, New |
| `SpecFrontmatter` | Parsed YAML frontmatter from a spec file |

### Structs & Enums

| Type | Description |
|------|-------------|
| `SpecAction` | Enum of subcommands: Check, Init, New |
| `SpecFrontmatter` | Parsed YAML frontmatter from a spec file |
| `ValidationResult` | Result of validating a single spec (warnings + errors) |
| `CheckReport` | Aggregate report from checking all specs |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(SpecAction) -> Result<()>` | Dispatches to check, init, or new |
| `check` | `(strict: bool) -> Result<()>` | Validates all specs and prints report |
| `init` | `() -> Result<()>` | Scaffolds `.specsync/` with config.toml, registry.toml, .gitignore, version |
| `new_spec` | `(name: &str) -> Result<()>` | Creates spec directory with spec.md and companion files |

## Invariants

1. `spec check` exits non-zero if any errors are found (or warnings in strict mode)
2. `spec init` refuses to overwrite an existing `.specsync/` directory
3. `spec new` refuses to overwrite an existing spec directory
4. Frontmatter must contain `module`, `version`, `status`, and `files` fields
5. All files listed in frontmatter `files` must exist on disk
6. All required sections from config must be present in the spec body
7. Companion files (requirements.md, tasks.md, context.md, testing.md) are validated if present

## Behavioral Examples

### spec check — all valid
```
$ fledge spec check
✓ init (v4, active) — 1 file, 7/7 sections
✓ config (v4, active) — 1 file, 7/7 sections
  2 specs checked, 0 errors, 0 warnings
```

### spec check — missing section
```
$ fledge spec check
✗ init (v4, active) — missing sections: Error Cases
  1 spec checked, 1 error, 0 warnings
```

### spec check — missing source file
```
$ fledge spec check
✗ config (v3, active) — file not found: src/old_config.rs
  1 spec checked, 1 error, 0 warnings
```

### spec check — strict mode with warnings
```
$ fledge spec check --strict
⚠ init (v4, active) — companion file missing: design.md
  1 spec checked, 0 errors, 1 warning (treated as error in strict mode)
```

### spec init — fresh project
```
$ fledge spec init
✓ Created .specsync/config.toml
✓ Created .specsync/registry.toml
✓ Created .specsync/.gitignore
✓ Created .specsync/version
✓ Created specs/
  Spec-sync initialized. Run `fledge spec new <name>` to create your first spec.
```

### spec new — scaffold a module spec
```
$ fledge spec new auth
✓ Created specs/auth/auth.spec.md
✓ Created specs/auth/requirements.md
✓ Created specs/auth/tasks.md
✓ Created specs/auth/context.md
✓ Created specs/auth/testing.md
  Spec module 'auth' created. Edit specs/auth/auth.spec.md to get started.
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `.specsync/config.toml` not found | `spec check` without init | Print helpful message suggesting `fledge spec init` |
| `.specsync/` already exists | `spec init` on initialized project | Bail with message |
| Spec directory already exists | `spec new <name>` where `specs/<name>/` exists | Bail with message |
| Invalid YAML frontmatter | Spec file has malformed frontmatter | Report as error, continue checking other specs |
| No specs found | `spec check` with empty specs directory | Print message, exit 0 |

## Dependencies

- `serde` / `serde_json` — frontmatter parsing
- `toml` — config reading/writing
- `walkdir` — spec directory traversal
- `console` — styled terminal output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec for fledge spec integration |
