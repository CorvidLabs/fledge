---
spec: review.spec.md
---

## Test Plan

### Unit Tests

- Default branch detection (main vs master)
- Empty diff detection
- Diff stat line parsing
- File filter flag applied to git diff command

### Integration Tests

- `fledge review` runs against a branch with changes (requires Claude CLI)
- `fledge review --file <path>` restricts diff to one file
- Empty diff produces a clear error message
