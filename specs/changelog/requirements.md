---
module: changelog
type: requirements
---

# Changelog Requirements

## User Stories

1. As a developer, I want to see a changelog generated from my git tags so I can quickly review what shipped in each release.
2. As a developer, I want to see unreleased changes so I know what will be in the next release.
3. As a developer, I want JSON output so I can pipe changelog data to other tools.
4. As a developer, I want commits grouped by type so I can scan features vs fixes at a glance.

## Durable Requirements

### REQ-changelog-001

The implementation SHALL satisfy the following criterion: `fledge changelog` shows all tagged releases with commits grouped by conventional commit type

Acceptance Criteria

- `fledge changelog` shows all tagged releases with commits grouped by conventional commit type

### REQ-changelog-002

The implementation SHALL satisfy the following criterion: `fledge changelog --limit 3` shows only the 3 most recent releases

Acceptance Criteria

- `fledge changelog --limit 3` shows only the 3 most recent releases

### REQ-changelog-003

The implementation SHALL satisfy the following criterion: `fledge changelog --tag v0.5.0` shows only that release

Acceptance Criteria

- `fledge changelog --tag v0.5.0` shows only that release

### REQ-changelog-004

The implementation SHALL satisfy the following criterion: `fledge changelog --unreleased` shows commits since the latest tag

Acceptance Criteria

- `fledge changelog --unreleased` shows commits since the latest tag

### REQ-changelog-005

The implementation SHALL satisfy the following criterion: `fledge changelog --json` outputs structured JSON

Acceptance Criteria

- `fledge changelog --json` outputs structured JSON

### REQ-changelog-006

The implementation SHALL satisfy the following criterion: Merge commits are excluded

Acceptance Criteria

- Merge commits are excluded

### REQ-changelog-007

The implementation SHALL satisfy the following criterion: Scoped commits (e.g., `fix(parser): msg`) are correctly parsed

Acceptance Criteria

- Scoped commits (e.g., `fix(parser): msg`) are correctly parsed

### REQ-changelog-008

The implementation SHALL satisfy the following criterion: Non-conventional commits appear under "Other"

Acceptance Criteria

- Non-conventional commits appear under "Other"

## Acceptance Criteria

- `fledge changelog` shows all tagged releases with commits grouped by conventional commit type
- `fledge changelog --limit 3` shows only the 3 most recent releases
- `fledge changelog --tag v0.5.0` shows only that release
- `fledge changelog --unreleased` shows commits since the latest tag
- `fledge changelog --json` outputs structured JSON
- Merge commits are excluded
- Scoped commits (e.g., `fix(parser): msg`) are correctly parsed
- Non-conventional commits appear under "Other"
