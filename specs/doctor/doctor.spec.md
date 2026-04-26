---
module: doctor
version: 7
status: active
files:
  - src/doctor.rs

db_tables: []
depends_on:
  - config
  - llm
---

# Doctor

## Purpose

Diagnoses fledge's own environment health: validates that fledge config loads, git is configured, the AI provider is reachable, and (informationally) which language toolchains are present. Provides actionable fix suggestions for failing checks.

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
| `Section` | A named group of checks; `informational` sections are excluded from pass/fail totals |
| `CheckResult` | Individual check: name, status, version, fix suggestion |
| `CheckStatus` | Enum: `Ok`, `Missing`, `Error` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(DoctorOptions) -> Result<()>` | Main entry — run checks, print report |

## Sections

The report has four sections, in this order:

| Section | Informational? | What it checks |
|---------|---------------|---------------|
| `fledge` | no | `fledge config` loads cleanly |
| `Git` | no | `git` installed; repository initialized; remote configured; working tree clean |
| `AI` | no | Claude CLI present; Ollama binary present; the active provider's reachability (probes Ollama's `/api/tags` when active) |
| `Toolchains` | **yes** | Probes for rust (rustc/cargo), node (node/npm/pnpm/bun/yarn), python (python3/uv/poetry), go, ruby, swift, and JVM (java/gradle/mvn) |

## Invariants

1. The `fledge`, `Git`, and `AI` sections contribute to the passed/failed totals; the `Toolchains` section does not (`section.informational == true` excludes it from counting and renders missing tools dimmed rather than as failures, since not every project uses every language).
2. Toolchain probes capture the tool's version string when present; missing tools render as `· <name> (not installed)` and carry no fix hint (environmental, not project errors).
3. AI checks verify Claude CLI is installed and Ollama (when configured as active) is reachable; when Ollama is the active provider the displayed `model` honors the `FLEDGE_AI_MODEL` env override so doctor's report matches what `ask` / `review` will actually send.
4. Each failing check in a non-informational section includes an actionable fix command.
5. `--json` outputs a structured `DoctorReport` with all sections (informational sections still appear in the JSON, with `informational: true`).
6. Exit summary shows count of passed checks and issues found, computed only over non-informational sections.
7. Tool version is extracted by running `<tool> --version` (or `version` for `go`, `-version` for `java`) and parsing the first version-like token, stripping `v` and `go` prefixes.
8. The `check_tool` helper enforces a 10-second per-probe timeout to keep `fledge doctor` fast even when a hung binary is in `PATH`.

## Behavioral Examples

```
$ fledge doctor

  fledge
    ✅ fledge config 0.15.1 — loaded

  Git
    ✅ git 2.50.1
    ✅ repository — initialized
    ✅ remote — origin ➡️ git@github.com:CorvidLabs/fledge.git
    ✅ working tree — clean

  AI
    ✅ claude 2.1.119
    ✅ ollama 0.21.2
    ✅ Active provider: ollama — ollama is the active provider (model: gpt-oss:120b-cloud, host: http://localhost:11434)

  Toolchains
    ✅ rustc 1.93.0
    ✅ cargo 1.93.0
    ✅ node 25.5.0
    ✅ bun 1.3.12
    · pnpm (not installed)
    · yarn (not installed)
    ✅ python3 3.14.3
    ✅ swift 6.3
    · go (not installed)

  7 checks passed, 0 issues found

$ fledge doctor --json
{ "sections": [...], "passed": 7, "failed": 0 }
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Cannot detect cwd | Current dir inaccessible | anyhow error |
| Tool probe times out | `<tool> --version` hangs more than 10s | Killed; reported as `Error` with `timed out after 10s` |

## Dependencies

- `std::process::Command` for running tool version checks
- `config` module — reads `ai.provider` and `ai.ollama.host` to determine the active LLM provider
- `llm` module — `resolve_provider_kind` for active-provider selection, `ProviderKind` for display
- `ureq` — probes the Ollama endpoint's `/api/tags` to check reachability
- `console` for styled output
- `serde` + `serde_json` for JSON output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 7 | 2026-04-26 | Tier-D 1.0 envelope: `doctor --json` now wraps output as `{schema_version: 1, action: "doctor", sections, passed, failed}`. Previously emitted bare `{sections, passed, failed}` — a 1.0 contract violation per the AGENTS.md rule that every `--json` output is enveloped. Inner `DoctorReport` struct unchanged so the unit test still validates section serialization. New integration assertion in `cli_doctor_json_valid` |
| 6 | 2026-04-25 | Re-absorbed `fledge-plugin-doctor` toolchain probes into core as a new informational `Toolchains` section. Missing toolchain entries render dimmed and don't pollute the pass/fail totals because environmental availability isn't a project error. Plugin dropped from `DEFAULT_PLUGINS`. |
| 5 | 2026-04-25 | v0.15 tight-core: stripped `Project Type`, `Toolchain`, and `Dependencies` sections. Self-check only: fledge config, git, AI provider. Toolchain probes deferred to `fledge-plugin-doctor`. |
| 4 | 2026-04-24 | Active-Ollama display honors `FLEDGE_AI_MODEL` env override (previously only `OLLAMA_HOST` was honored) |
| 3 | 2026-04-23 | AI section reports both Claude CLI and Ollama binary presence, the active provider (from config / env), and probes the Ollama host's `/api/tags` endpoint for reachability |
| 2 | 2026-04-21 | Add swift to supported project types |
| 1 | 2026-04-20 | Initial spec |
