---
spec: spinner.spec.md
---

## Key Decisions

- Random theme selection per invocation for visual variety (no user preference stored)
- Platform-aware RNG: `/dev/urandom` on Unix, `SystemTime` nanos elsewhere — avoids adding a `rand` dependency
- All themes end with a blank space frame so `finish_and_clear` leaves clean terminal output
- Spinner displays message before animation (`{msg} {spinner}`) with 2-space indent for visual nesting
- 10 built-in themes balancing emoji (5) and ASCII/Unicode (5) for terminal compatibility

## Files to Read First

- `src/spinner.rs` — complete implementation (single file, ~100 LOC)
- `specs/spinner/spinner.spec.md` — formal API and invariants

## Current Status

- Fully implemented and used by 13 modules (checks, work, prs, issues, lanes, search, publish, update, plugin, review, ask)
- No configuration or user-facing options — purely internal UX detail
- Spec at v1

## Notes

- The spinner is intentionally simple — no progress percentage, no ETA, just animated feedback that something is happening
- Theme selection is non-deterministic by design; tests that need determinism should test `random_index` bounds, not specific themes
