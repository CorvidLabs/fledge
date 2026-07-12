---
spec: watch.spec.md
---

## Requirements

- Watch a directory recursively for filesystem changes
- Re-run a specified task or lane when relevant files change
- Filter events by file extension (optional)
- Ignore common non-source directories (.git, target, node_modules, .fledge, __pycache__)
- Configurable debounce interval with deadline extension on new events
- Perform an initial run before entering the watch loop
- Optional terminal clear before each re-run
- Graceful error handling — target failures don't stop the watcher

## Durable Requirements

### REQ-watch-001

The implementation SHALL satisfy the following criterion: Watch a directory recursively for filesystem changes

Acceptance Criteria

- Watch a directory recursively for filesystem changes

### REQ-watch-002

The implementation SHALL satisfy the following criterion: Re-run a specified task or lane when relevant files change

Acceptance Criteria

- Re-run a specified task or lane when relevant files change

### REQ-watch-003

The implementation SHALL satisfy the following criterion: Filter events by file extension (optional)

Acceptance Criteria

- Filter events by file extension (optional)

### REQ-watch-004

The implementation SHALL satisfy the following criterion: Ignore common non-source directories (.git, target, node_modules, .fledge, __pycache__)

Acceptance Criteria

- Ignore common non-source directories (.git, target, node_modules, .fledge, __pycache__)

### REQ-watch-005

The implementation SHALL satisfy the following criterion: Configurable debounce interval with deadline extension on new events

Acceptance Criteria

- Configurable debounce interval with deadline extension on new events

### REQ-watch-006

The implementation SHALL satisfy the following criterion: Perform an initial run before entering the watch loop

Acceptance Criteria

- Perform an initial run before entering the watch loop

### REQ-watch-007

The implementation SHALL satisfy the following criterion: Optional terminal clear before each re-run

Acceptance Criteria

- Optional terminal clear before each re-run

### REQ-watch-008

The implementation SHALL satisfy the following criterion: Graceful error handling — target failures don't stop the watcher

Acceptance Criteria

- Graceful error handling — target failures don't stop the watcher
