---
change: CHG-0003-fix-bare-fledge-exit-code-and-restore-cursor-on-ctrl-c
artifact: requirements
---

# Requirements

- Bare `fledge` invocation must print the top-level help and exit 0.
- `fledge --help` and `fledge help` must continue to exit 0 and print help unchanged.
- `fledge plugins search --interactive` and other dialoguer prompts must not leave the terminal cursor hidden after the user interrupts with Ctrl+C.
- Ctrl+C during interactive prompts must continue to exit 130, matching standard shell convention.
- Non-interactive commands must not be affected by the cursor-restoration handler.
