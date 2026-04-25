---
spec: doctor.spec.md
---

## Context

`fledge doctor` helps users diagnose why commands might fail — fledge config that won't load, broken git config, absent AI tooling, or missing language toolchains. It follows the convention of tools like `brew doctor` and `flutter doctor`, providing actionable fix commands rather than just error messages.

The four sections (`fledge`, `Git`, `AI`, `Toolchains`) split cleanly along a "is this a project error?" axis. The first three count toward pass/fail because their failures break fledge itself or its provider; the `Toolchains` section is informational because environmental absence (no Swift compiler installed) isn't a defect for a project that doesn't use Swift.

## Related Modules

- `config` — `fledge config` loads as the first health signal
- `llm` — used to determine the active AI provider for the AI section
- `ureq` — probes the Ollama endpoint's `/api/tags` to distinguish "daemon down" from "not installed"

## Design Decisions

- Checks run tool `--version` commands rather than probing PATH — captures version info alongside existence
- Each failing non-informational check includes a concrete fix command to minimize user friction
- The `Toolchains` section probes 16 binaries unconditionally rather than gating on detected project type. Project-type detection was tried in earlier versions; it produced false negatives (e.g. polyglot repos) and surprised users when the Swift toolchain wasn't reported on a Rust+Swift codebase. Reporting all toolchains as info is simpler and clearer.
- Missing toolchains render dimmed (`· tool (not installed)`) rather than as red ❌ to signal "not a problem unless you need this"
- A 10-second per-probe timeout in `check_tool` keeps the report fast even when a hung binary is on `PATH`
