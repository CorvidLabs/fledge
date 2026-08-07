---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
artifact: design
---

# Design

1. **Bare invocation exit 0.** The top-level `Cli` struct now uses `Option<Commands>` for its subcommand field. When parsing produces `None`, `main.rs` prints the top-level help and returns `Ok(())`, producing exit code 0. This replaces clap's default behavior for a required subcommand, which printed help but exited 2.

2. **Cursor restoration on Ctrl+C.** Dialoguer-based prompts hide the terminal cursor while rendering. If the process is killed by SIGINT before the prompt can restore it, the cursor stays hidden. A one-shot Ctrl+C handler is installed via the `ctrlc` crate whenever stdin is a TTY. The handler shows the cursor with `console::Term::stdout().show_cursor()` and then exits with code 130, preserving the standard shell convention for an interrupted process.

Both fixes are localized: the first touches `src/cli.rs` and `src/main.rs`; the second adds a helper in `src/utils.rs` and invokes it from `src/main.rs`.
