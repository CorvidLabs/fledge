---
spec: doctor.spec.md
---

## User Stories

- As a developer, I want to run `fledge doctor` to check if my development environment is properly set up
- As a CI system, I want to run `fledge doctor --json` to get machine-readable health data

## Acceptance Criteria

- `fledge doctor` detects the project type and checks relevant toolchain, dependencies, and git state
- Each failing check shows an actionable fix command
- `--json` outputs a structured `DoctorReport` with all check results
- Exit summary shows count of passed checks and issues found
- Supported project types: rust, node, go, python, ruby, java-gradle, java-maven, generic

## Constraints

- Depends on `run::detect_project_type` for project ecosystem detection
- Tool version detection runs `<tool> --version` and parses the output

## Out of Scope

- Auto-fixing issues (only suggestions)
- Remote dependency health checks
- Network connectivity checks
