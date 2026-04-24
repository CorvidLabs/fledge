---
spec: review.spec.md
---

## Tasks

- [x] Write spec files for review module
- [x] Implement diff generation against base branch
- [x] Implement Claude CLI invocation with diff as input
- [x] Add `--base` and `--file` flags
- [x] Add diff stat display
- [x] Wire up CLI subcommand in main.rs
- [x] Write unit tests
- [x] Run verification suite
- [x] Auto-detect spec context from diff changed-file list (match frontmatter `files:` and `specs/<name>/` prefix)
- [x] `--with-specs <names>` to append explicit modules
- [x] `--no-auto-specs` to disable auto-detection
- [x] Add `spec_context` field to JSON output so agents can see which specs were in prompt
- [x] Constrain prompt: specs are context-only, review target is the diff, no comments on unchanged code
- [x] Show `Spec context: trust, work` line in pretty output when specs were included

## Gaps

- No token-budget guardrail: a huge diff touching many modules could balloon the prompt. In practice diffs are small and specs are small — revisit if this bites
- No way to include *only a section* of a spec (e.g. just `## Invariants`) — whole-bundle or nothing
- `review` still doesn't review the review diff itself (meta) — the prompt tells Claude to ignore the specs as review targets, but if this file-pair IS the diff (e.g. reviewing changes to specs/review), auto-detect will include review's own spec. That's intentional: it gives Claude the invariants it should check against
