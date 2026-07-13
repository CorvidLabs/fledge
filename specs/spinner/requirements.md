---
spec: spinner.spec.md
---

## User Stories

- As a user, I want visual feedback during long-running operations so I know the CLI hasn't frozen
- As a user, I want the spinner to clean up after itself so my terminal isn't cluttered

## Acceptance Criteria

### REQ-spinner-001

The implementation SHALL meet this contract: `Spinner::start(msg)` displays an animated spinner with the given message

### REQ-spinner-002

The implementation SHALL meet this contract: `Spinner::finish()` clears the spinner line completely

### REQ-spinner-003

The implementation SHALL meet this contract: A random theme is chosen each time a spinner starts

### REQ-spinner-004

The implementation SHALL meet this contract: All themes animate smoothly without visual glitches

## Constraints

- No external RNG dependency — use platform primitives only
- Must work in both emoji-capable and basic terminals (mix of theme types)
- Spinner must not interfere with subsequent terminal output after `finish()`

## Out of Scope

- User-selectable spinner theme
- Progress bar mode (percentage/ETA)
- Nested or concurrent spinners
- Color/style customization
