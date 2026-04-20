---
spec: tui.spec.md
---

## User Stories

- As a developer, I want to run `fledge tui` and browse all available commands without memorizing CLI flags
- As a new user, I want to discover fledge's capabilities through an interactive menu
- As a developer, I want to run commands and see output inline without leaving the TUI

## Acceptance Criteria

- `fledge tui` launches a two-panel dashboard with categories on the left and actions on the right
- All 11 categories are present: Work, GitHub, Run, Specs, Metrics, Config, Templates, AI, Doctor, Changelog, Plugins
- Direct actions run immediately and show output in a scrollable panel
- Input-requiring actions show an inline form with labeled fields, defaults, and required field validation
- Template browser launches as a nested TUI and returns to the dashboard on exit
- Keyboard navigation works: arrows/j/k, Tab, Enter, Esc, q
- Terminal state is always restored on exit (raw mode disabled, alternate screen left)

## Constraints

- Requires `--features tui` at compile time (ratatui + crossterm are optional dependencies)
- Commands run as subprocesses — requires the fledge binary to be available via `current_exe()`

## Out of Scope

- Concurrent command execution
- Command history / recent commands
- Custom keybindings
- Mouse support
