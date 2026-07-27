---
id: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
state: accepted
type: bug_fix
base_commit: fb6260ec05933dc8e11018bf6cfb66d482161078
---

# Fix bare fledge exit code and restore cursor on Ctrl+C

## Intent

Fix bare fledge exit code and restore cursor on Ctrl+C

## Affected Canonical Specs

- `main`
- `utils`

## Acceptance Criteria

- Bare fledge invocation exits 0 and prints help; Ctrl+C during fledge plugins search --interactive exits 130 and leaves the terminal cursor visible

## No-spec Rationale

Not applicable
