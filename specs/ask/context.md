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
- v3 adds a default-on spec index to every prompt. Rationale: the specs are the richest source of *why* in the repo, and `ask` without them was strictly worse than a generic Claude chat in a project with strong documentation. The index is small (~one line per module) so it barely affects cost but materially changes the quality of answers.
- `--with-specs` uses the same `spec::load_module_bundle` helper that `spec show` will eventually consume. Keeping this logic in the `spec` module means any future change to bundle format (e.g. filtering long sections) is a single-file change.
- `"all"` expansion is guarded by the fact that fledge projects have ~32 specs today; a truly massive spec tree would warrant pagination, but that's premature.
- `--no-spec-index` exists for the narrow case where a user wants to ask a pure syntax/API question unrelated to the repo — saves a few hundred tokens on the prompt prefix.
