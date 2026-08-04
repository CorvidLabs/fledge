---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
module: main
---

## ADDED

### REQUIREMENT REQ-main-005

The CLI SHALL print the top-level help and exit 0 when invoked with no subcommand.

Acceptance Criteria
- `fledge` with no arguments exits 0 and stdout contains "Usage".
- `tests/templates.rs::cli_no_args_shows_help` asserts success.

### REQUIREMENT REQ-main-006

Interactive dialoguer-based prompts SHALL not leave the terminal cursor hidden after the user interrupts with Ctrl+C.

Acceptance Criteria
- `fledge plugins search --interactive` followed by Ctrl+C exits 130 and the shell cursor remains visible.
