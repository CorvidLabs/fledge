---
spec: tui.spec.md
---

## Test Plan

### Unit Tests

- `build_categories` returns 11 categories
- `build_command` produces correct args for each `ActionId` variant
- `strip_ansi` removes ANSI escape sequences from strings
- All `ActionId` variants are handled in `build_command` (no missing match arms)

### Integration Tests

- `fledge tui` compiles only with `--features tui`
- TUI code is absent from non-TUI builds (feature gate works)
