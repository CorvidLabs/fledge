---
spec: spinner.spec.md
---

## Test Plan

### Unit Tests

- `random_index_within_bounds` — result is always `< max` for various max values
- `random_index_with_one` — always returns 0 when max is 1
- `all_themes_end_with_blank` — every theme's last frame is `" "` (space)
- `all_themes_have_minimum_frames` — every theme has at least 3 frames
- `all_themes_have_valid_interval` — interval is within 80–300ms range
- `theme_count` — exactly 10 themes are defined
- `spinner_start_finish` — spinner can be created and finished without panic

### Integration Tests

- Spinner is exercised implicitly by all commands that make network calls (checks, work, prs, issues, search, publish, review, ask, lanes, plugin, update)

### Manual Testing

```bash
# Any network command shows a spinner
fledge checks
fledge prs
fledge search rust

# Spinner clears cleanly — no leftover text after command completes
```
