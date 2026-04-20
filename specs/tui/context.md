---
spec: tui.spec.md
---

## Context

`fledge tui` is the interactive companion to the CLI. Instead of memorizing flags and subcommands, users navigate a categorized menu and run any fledge command inline. Originally a template-only browser, it was expanded to a full dev-lifecycle dashboard covering all 11 command categories.

## Related Modules

- `templates` — provides `discover_templates_with_repos` and `render_template` for the nested template browser
- `init` — provides `init_git_for_tui` for git init after scaffolding
- `config` — provides `Config` for detecting project configuration

## Design Decisions

- Commands run as subprocesses (`std::env::current_exe()`) rather than calling module functions directly — this avoids coupling TUI to every module's internal API and ensures identical behavior to CLI usage
- ANSI codes are stripped from subprocess output since ratatui handles its own styling
- The template browser is a separate nested TUI that takes over the full screen, rather than being embedded in the dashboard — this preserves the original template browser experience
- Feature-gated behind `tui` to keep the binary small for users who don't need it
