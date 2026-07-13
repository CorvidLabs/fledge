---
spec: watch.spec.md
---

## Requirements

### REQ-watch-001

The implementation SHALL meet this contract: Watch a directory recursively for filesystem changes

### REQ-watch-002

The implementation SHALL meet this contract: Re-run a specified task or lane when relevant files change

### REQ-watch-003

The implementation SHALL meet this contract: Filter events by file extension (optional)

### REQ-watch-004

The implementation SHALL meet this contract: Ignore common non-source directories (.git, target, node_modules, .fledge, __pycache__)

### REQ-watch-005

The implementation SHALL meet this contract: Configurable debounce interval with deadline extension on new events

### REQ-watch-006

The implementation SHALL meet this contract: Perform an initial run before entering the watch loop

### REQ-watch-007

The implementation SHALL meet this contract: Optional terminal clear before each re-run

### REQ-watch-008

The implementation SHALL meet this contract: Graceful error handling — target failures don't stop the watcher
