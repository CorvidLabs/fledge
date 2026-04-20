---
module: doctor
version: 1
status: active
files:
  - src/doctor.rs

db_tables: []
depends_on:
  - run
---

# Doctor

## Purpose

Diagnoses project environment health by checking toolchain availability, dependency state, and git configuration. Provides actionable fix suggestions for each failing check.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — run all diagnostic checks and print results |
| `DoctorOptions` | Options: `json` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `DoctorOptions` | CLI options: `json` |
| `DoctorReport` | Serializable report with all check results |
| `CheckResult` | Individual check: name, status, version, fix suggestion |
| `CheckStatus` | Enum: `Ok`, `Missing`, `Error` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(DoctorOptions) -> Result<()>` | Main entry — run checks, print report |

## Invariants

1. Project type is detected via `run::detect_project_type`
2. Toolchain checks verify required tools exist and capture their version strings
3. Dependency state checks verify lock files and install directories exist
4. Git checks verify git is installed, repo is initialized, and remote is configured
5. AI checks verify Claude CLI is installed and report availability of AI commands (fledge review, fledge ask)
6. Each failing check includes an actionable fix command
7. `--json` outputs a structured `DoctorReport`
8. Exit summary shows count of passed checks and issues found
9. Tool version is extracted by running `<tool> --version` and parsing first version-like string
10. Supported project types: rust, node, go, python, ruby, java-gradle, java-maven, generic

## Behavioral Examples

```
$ fledge doctor

  Toolchain
    ✓ rustc 1.78.0
    ✓ cargo 1.78.0
    ✗ cargo-clippy — not found
      → rustup component add clippy

  Dependencies
    ✓ Cargo.lock found
    ✓ target/ exists

  Git
    ✓ git 2.44.0
    ✓ remote: origin → https://github.com/...
    ✗ uncommitted changes (3 files)

  AI
    ✓ claude 1.x.x
    ✓ AI commands — fledge review, fledge ask available

  3 checks passed, 2 issues found

$ fledge doctor --json
{ "project_type": "rust", "sections": [...], "passed": 3, "failed": 2 }
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Cannot detect cwd | Current dir inaccessible | anyhow error |

## Dependencies

- `run::detect_project_type` for ecosystem detection
- `std::process::Command` for running tool version checks
- `console` for styled output
- `serde` + `serde_json` for JSON output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
