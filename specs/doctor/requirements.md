---
spec: doctor.spec.md
---

## User Stories

- As a developer, I want to run `fledge doctor` to check if my development environment is properly set up
- As a CI system, I want to run `fledge doctor --json` to get machine-readable health data
- As a polyglot developer, I want a single command to inventory which language toolchains are installed without false-failing on languages my current project doesn't use

## Acceptance Criteria

### REQ-doctor-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge doctor` reports four sections: `fledge`, `Git`, `AI`, `Toolchains`
### REQ-doctor-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Each failing check in a non-informational section shows an actionable fix command
### REQ-doctor-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- The `Toolchains` section is informational — missing entries render dimmed and don't pollute the pass/fail totals
### REQ-doctor-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `--json` outputs a structured `DoctorReport` with all check results, including `informational: bool` per Section
### REQ-doctor-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Exit summary shows count of passed checks and issues found, computed only over non-informational sections
### REQ-doctor-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Toolchains probed: rustc, cargo, node, npm, pnpm, bun, yarn, python3, uv, poetry, go, ruby, swift, java, gradle, mvn

## Constraints

- Tool version detection runs `<tool> --version` (or the tool's equivalent — `version` for `go`, `-version` for `java`) and parses the first version-like token
- Each probe enforces a 10-second timeout to bound report time
- AI section probes the Ollama host's `/api/tags` to distinguish "daemon down" from "not installed"

## Out of Scope

- Auto-fixing issues (only suggestions)
- Remote dependency health checks (use `fledge-plugin-deps`)
- Per-project lockfile/build-artifact checks (removed in v0.15)
