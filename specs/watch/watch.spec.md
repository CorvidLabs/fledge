---
module: watch
version: 1
status: active
files:
  - src/watch.rs

db_tables: []
depends_on:
  - run
  - lanes
---

# Watch

## Purpose

File watcher that monitors a directory for changes and automatically re-runs a specified task or lane. Uses the `notify` crate for cross-platform filesystem events with configurable debouncing, extension filtering, and path ignore patterns.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — starts watching and re-running |
| `WatchOptions` | Options: `name`, `lane`, `path`, `extensions`, `debounce_ms`, `clear` |
| `should_ignore_path` | Check if a path falls under an ignored directory |
| `matches_extensions` | Check if a path matches the extension filter |
| `parse_extensions` | Parse comma-separated extension string into a list |

### Structs & Enums

| Type | Description |
|------|-------------|
| `WatchOptions` | Configuration: target name, lane mode, watch path, extension filter, debounce interval, clear flag |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(WatchOptions) -> Result<()>` | Main entry — watch directory and re-run target on changes |
| `should_ignore_path` | `(&Path) -> bool` | Returns true if path is under an ignored directory (.git, target, node_modules, .fledge, __pycache__) |
| `matches_extensions` | `(&Path, &[String]) -> bool` | Returns true if path matches extension filter (empty filter matches all) |
| `parse_extensions` | `(&str) -> Vec<String>` | Parses comma-separated extension string, strips dots and whitespace |

## Invariants

1. Watches recursively from the specified path (or cwd if none given)
2. Ignores events from `.git`, `target`, `node_modules`, `.fledge`, and `__pycache__` directories
3. Only triggers on Create, Modify, and Remove events
4. Extension filter is optional — empty filter matches all files
5. Debounce window defaults to 500ms; each new relevant event extends the deadline
6. Performs an initial run before entering the watch loop
7. `--clear` clears the terminal before each re-run
8. `--lane` flag switches from task mode to lane mode
9. Errors during target execution are printed but do not stop the watcher

## Behavioral Examples

```
# Watch and re-run a task
$ fledge watch test
* Watching for changes to re-run task test
  Path: /project
  Debounce: 500ms
  Press Ctrl+C to stop.

>>> Re-running task: test

OK Completed in 1.032s
Watching for changes...

# Watch with extension filter
$ fledge watch test --ext rs,toml
* Watching for changes to re-run task test
  Path: /project
  Extensions: rs, toml
  Debounce: 500ms

# Watch a lane
$ fledge watch ci --lane
* Watching for changes to re-run lane ci

# Watch a specific directory with custom debounce
$ fledge watch build --path src --debounce 1000

# Watch with terminal clear
$ fledge watch test --clear
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Watch path not found | Specified `--path` doesn't exist | Error with path |
| Watcher creation failed | OS-level filesystem watch error | Error with context |
| Target execution failed | Task/lane exits non-zero | Print error, continue watching |
| Channel disconnected | Watcher dropped unexpectedly | Exit gracefully |

## Dependencies

- `run` module (task execution)
- `lanes` module (lane execution)
- `notify` crate (filesystem events)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-23 | Initial spec |
