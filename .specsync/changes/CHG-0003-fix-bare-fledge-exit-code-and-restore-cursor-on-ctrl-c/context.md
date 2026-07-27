---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
artifact: context
---

# Context

Two small but persistent CLI UX issues were reported: running `fledge` with no subcommand always returned clap usage-error exit code 2, and hitting Ctrl+C during `fledge plugins search --interactive` left the terminal cursor hidden (exit 130). The first violates the common convention that a bare CLI invocation prints help and exits 0; the second leaves users with a broken shell prompt after an otherwise normal interruption. This change fixes both without altering any command semantics.
