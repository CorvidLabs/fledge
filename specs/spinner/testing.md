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

- Spinner is exercised implicitly by every command that makes a network or git call: `templates search`/`publish`, `lanes search`/`publish`/`import`, `plugins search`/`install`/`update`/`publish`, `work pr`/`status`, `review`, `ask`, `ai models` (Ollama). Plugin commands (`checks`, `prs`, `issues`, `deps`, `metrics`) also reuse the same spinner via the protocol.

### Manual Testing

```bash
# Any network command shows a spinner
fledge templates search rust
fledge plugins search deploy
fledge work pr

# Spinner clears cleanly — no leftover text after command completes
```
