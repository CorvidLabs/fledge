---
module: spinner
version: 1
status: active
files:
  - src/spinner.rs

db_tables: []
depends_on: []
---

# Spinner

## Purpose

Provide a consistent loading spinner for long-running operations (network calls, git pushes, etc.). Randomly selects from 10 visual themes on each invocation for personality.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `Spinner` | Loading spinner with random theme selection |
| `start` | Create and start a spinner with a message (`Spinner::start`) |
| `finish` | Stop the spinner and clear its line (`Spinner::finish`) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `Spinner` | Wraps `indicatif::ProgressBar` with themed animation |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `Spinner::start` | `(&str) -> Spinner` | Create and start a spinner with the given message |
| `Spinner::finish` | `(&self) -> ()` | Stop the spinner and clear its line |

## Internal Details

### Theme

| Field | Type | Description |
|-------|------|-------------|
| `frames` | `&'static [&'static str]` | Animation frame sequence (last frame is blank for clean finish) |
| `interval_ms` | `u64` | Milliseconds between frame ticks |

### Constants

| Constant | Description |
|----------|-------------|
| `THEMES` | 10 spinner themes: clock emoji, rock-paper-scissors, moon phases, weather, globe, 5 braille/block/arrow variants |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `random_index` | `(usize) -> usize` | Platform-aware RNG: `/dev/urandom` on Unix, `SystemTime` nanos on other platforms |

## Invariants

1. Every theme's last frame is a single space (`" "`) so `finish_and_clear` leaves no artifacts
2. `random_index` never panics — fallback to 0 if RNG source fails (all-zero buf → `0 % max`)
3. Spinner template is `"  {msg} {spinner}"` — two-space indent, message before animation
4. All themes have at least 3 frames
5. Interval range is 80–300ms across all themes

## Behavioral Examples

```
# Spinner appears during any network/git operation
$ fledge templates search rust
  Searching GitHub for community templates: ⠙    # animates through braille frames

$ fledge work pr
  Pushing feat/spinner to origin: 🌎    # animates through globe frames
  ✅ Pushed to origin/feat/spinner
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| None | Spinner is infallible | `start` always succeeds; `finish` always clears |

## Dependencies

- `indicatif` — progress bar / spinner rendering

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
