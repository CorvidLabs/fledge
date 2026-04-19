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
