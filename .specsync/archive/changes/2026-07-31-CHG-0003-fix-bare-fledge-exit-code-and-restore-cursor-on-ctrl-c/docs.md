---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
artifact: docs
---

# Docs

- `specs/main/main.spec.md` updated to version 12: documented optional top-level subcommand, bare-invocation behavioral example, and terminal-cursor restoration invariant.
- `specs/utils/utils.spec.md` updated to version 2: added `install_terminal_restore_handler` to the public API table, documented `ctrlc` and `console` dependencies, and added the idempotent cursor-restoration invariant.
