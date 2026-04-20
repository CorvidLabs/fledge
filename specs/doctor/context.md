---
spec: doctor.spec.md
---

## Context

`fledge doctor` helps users diagnose why commands might fail — missing tools, broken git config, absent lock files. It follows the convention of tools like `brew doctor` and `flutter doctor`, providing actionable fix commands rather than just error messages.

## Related Modules

- `run` — provides `detect_project_type` used to determine which toolchain checks to run

## Design Decisions

- Checks run tool `--version` commands rather than probing PATH — captures version info alongside existence check
- Each check includes a concrete fix command to minimize user friction
- AI checks are separate from toolchain — fledge works without Claude, but AI commands need it
