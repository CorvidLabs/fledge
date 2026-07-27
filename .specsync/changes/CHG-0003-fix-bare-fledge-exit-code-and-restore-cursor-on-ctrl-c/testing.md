---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
artifact: testing
---

# Testing

## REQ-main-005: bare `fledge` exits 0 and prints help

- Automated: `tests/templates.rs::cli_no_args_shows_help` asserts success and stdout containing "Usage".
- Manual: run `fledge` with no arguments; expect help on stdout and exit code 0.

## REQ-main-006: Ctrl+C during interactive prompts keeps cursor visible

- Manual: run `fledge plugins search --interactive`, wait for the fuzzy selector, then press Ctrl+C; expect exit code 130 and the shell cursor to remain visible.

## REQ-utils-010: `install_terminal_restore_handler` is idempotent

- Code review: the helper uses `std::sync::Once` so only the first call installs the handler.
- Manual: interrupt two different interactive prompts in one process; the handler is installed once and cursor is restored each time.

## Regression coverage

- `fledge --help` and `fledge help` continue to exit 0.
- Non-interactive commands (e.g., `fledge --version`, `fledge introspect --json`) are unaffected by the cursor-restoration handler.
- `fledge lanes run verify-native` (fmt, lint, test-governance, build, validate-templates) must pass.
