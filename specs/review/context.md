---
spec: review.spec.md
---

## Context

`fledge review` provides AI-powered code review as a pre-PR quality gate. Instead of waiting for human review feedback, developers can get immediate actionable suggestions on their diff. The module shells out to the Claude CLI, which has project context awareness.

## Related Modules

- `ask` — also uses Claude CLI; `review` is diff-focused while `ask` is question-focused

## Design Decisions

- Shell out to `claude` CLI rather than calling the API directly — leverages Claude's project context and avoids managing API keys separately
- Show diff stats before AI output so the developer knows the scope of the review
- Auto-detect default branch (main vs master) rather than hardcoding
- **v5: spec-awareness is auto-detected, not opt-in**. Rationale: the most common case is "I changed files in module X; tell me what I got wrong." Making the user manually pick specs defeats the point. Auto-detect uses two signals: (1) any file in the diff matches a spec's frontmatter `files:`, (2) any file in the diff is under `specs/<name>/`. Either triggers inclusion.
- **The review target is always the diff**. The prompt explicitly forbids Claude from critiquing the specs themselves or suggesting changes to unmodified code. This matches user intent: "review my changes" — specs answer the question "what were these changes *supposed* to do?" but are not on trial.
- `--no-auto-specs` exists as an escape hatch for the rare case of reviewing a diff in a project with stale specs, where the spec context would be actively misleading
- `--with-specs` mirrors the flag on `fledge ask` so the two commands feel consistent
- The JSON output now includes `spec_context: [...]` so agents and tooling can see which specs shaped the review without re-running the command
