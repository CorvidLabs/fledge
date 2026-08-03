---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
module: utils
---

## ADDED

### REQUIREMENT REQ-utils-010

The `utils` module SHALL provide an idempotent helper that installs a Ctrl+C handler to restore the terminal cursor before exiting 130.

Acceptance Criteria
- `install_terminal_restore_handler` is callable from `main.rs` when stdin is a TTY.
- Calling the helper multiple times is a no-op after the first installation.
