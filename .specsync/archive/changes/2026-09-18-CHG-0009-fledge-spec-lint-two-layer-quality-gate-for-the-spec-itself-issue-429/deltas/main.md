---
change: CHG-0009-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
module: main
---

## ADDED

### REQUIREMENT REQ-main-010

`main` SHALL dispatch the `spec lint` subcommand and propagate its exit status.

Acceptance Criteria
- `fledge spec lint` is reachable from the CLI and appears in `fledge introspect --json`.
- A clean run exits 0; a run with any error finding exits 1.
- Errors are written to stderr as plain text even when `--json` is active.
