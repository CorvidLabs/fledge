---
spec: review.spec.md
---

## User Stories

- As a developer, I want to run `fledge review` to get AI feedback on my branch changes before opening a PR
- As a developer, I want to review changes against a specific base branch
- As a developer, I want to review a single file's changes

## Durable Requirements

### REQ-review-001

The implementation SHALL satisfy the following criterion: `fledge review` diffs the current branch against the default base (main/master) and sends it to Claude CLI

Acceptance Criteria

- `fledge review` diffs the current branch against the default base (main/master) and sends it to Claude CLI

### REQ-review-002

The implementation SHALL satisfy the following criterion: `fledge review --base develop` uses a custom base branch

Acceptance Criteria

- `fledge review --base develop` uses a custom base branch

### REQ-review-003

The implementation SHALL satisfy the following criterion: `fledge review --file src/foo.rs` restricts the review to one file

Acceptance Criteria

- `fledge review --file src/foo.rs` restricts the review to one file

### REQ-review-004

The implementation SHALL satisfy the following criterion: Diff stats are displayed before the AI output

Acceptance Criteria

- Diff stats are displayed before the AI output

### REQ-review-005

The implementation SHALL satisfy the following criterion: Empty diffs bail with a clear message

Acceptance Criteria

- Empty diffs bail with a clear message

### REQ-review-006

The implementation SHALL satisfy the following criterion: Missing Claude CLI produces install instructions

Acceptance Criteria

- Missing Claude CLI produces install instructions

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
