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
