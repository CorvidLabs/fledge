---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
artifact: tasks
---

# Tasks

- [x] Reproduce bare `fledge` exit code 2 and confirm help is printed to stdout.
- [x] Change `Cli::command` to `Option<Commands>` and handle `None` by printing help and returning `Ok(())`.
- [x] Add one-shot Ctrl+C handler via `ctrlc` that restores the terminal cursor and exits 130.
- [x] Update `tests/templates.rs::cli_no_args_shows_help` to assert success.
- [x] Update `specs/main` and `specs/utils` with new invariants, dependencies, and changelog entries.
- [x] Create and populate SpecSync change CHG-0003.
