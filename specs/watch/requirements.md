---
spec: watch.spec.md
---

## Requirements

### REQ-watch-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Watch a directory recursively for filesystem changes
### REQ-watch-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Re-run a specified task or lane when relevant files change
### REQ-watch-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Filter events by file extension (optional)
### REQ-watch-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Ignore common non-source directories (.git, target, node_modules, .fledge, __pycache__)
### REQ-watch-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Configurable debounce interval with deadline extension on new events
### REQ-watch-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Perform an initial run before entering the watch loop
### REQ-watch-007

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Optional terminal clear before each re-run
### REQ-watch-008

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Graceful error handling — target failures don't stop the watcher
