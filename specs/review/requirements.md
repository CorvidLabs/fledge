---
spec: review.spec.md
---

## User Stories

- As a developer, I want to run `fledge review` to get AI feedback on my branch changes before opening a PR
- As a developer, I want to review changes against a specific base branch
- As a developer, I want to review a single file's changes

## Acceptance Criteria

### REQ-review-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge review` diffs the current branch against the default base (main/master) and sends it to Claude CLI
### REQ-review-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge review --base develop` uses a custom base branch
### REQ-review-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge review --file src/foo.rs` restricts the review to one file
### REQ-review-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Diff stats are displayed before the AI output
### REQ-review-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Empty diffs bail with a clear message
### REQ-review-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Missing Claude CLI produces install instructions

## Constraints

- Requires Claude CLI (`claude`) installed and authenticated
- Requires git CLI
- Output is streamed from the Claude process

## Out of Scope

- Posting review comments to GitHub
- Reviewing uncommitted changes (working tree)
- Configurable review prompts
