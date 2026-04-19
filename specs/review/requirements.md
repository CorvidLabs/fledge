---
spec: review.spec.md
---

## User Stories

- As a developer, I want to run `fledge review` to get AI feedback on my branch changes before opening a PR
- As a developer, I want to review changes against a specific base branch
- As a developer, I want to review a single file's changes

## Acceptance Criteria

- `fledge review` diffs the current branch against the default base (main/master) and sends it to Claude CLI
- `fledge review --base develop` uses a custom base branch
- `fledge review --file src/foo.rs` restricts the review to one file
- Diff stats are displayed before the AI output
- Empty diffs bail with a clear message
- Missing Claude CLI produces install instructions

## Constraints

- Requires Claude CLI (`claude`) installed and authenticated
- Requires git CLI
- Output is streamed from the Claude process

## Out of Scope

- Posting review comments to GitHub
- Reviewing uncommitted changes (working tree)
- Configurable review prompts
