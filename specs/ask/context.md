---
spec: ask.spec.md
---

## Context

`fledge ask` is the simplest AI integration — a single question, a single answer. It lets developers query their codebase without leaving the terminal. The Claude CLI provides project-aware context automatically, so no file selection is needed.

## Related Modules

- `review` — also uses Claude CLI; `ask` is freeform while `review` is diff-focused

## Design Decisions

- Join args without requiring quotes — `fledge ask how does X work` is more natural than `fledge ask "how does X work"`
- Shell out to `claude` CLI for project context awareness rather than calling the API directly
- No conversation state — each invocation is stateless to keep the implementation simple
