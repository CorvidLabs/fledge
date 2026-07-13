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

## Acceptance Criteria

### REQ-changelog-001

The implementation SHALL meet this contract: `fledge changelog` shows all tagged releases with commits grouped by conventional commit type

### REQ-changelog-002

The implementation SHALL meet this contract: `fledge changelog --limit 3` shows only the 3 most recent releases

### REQ-changelog-003

The implementation SHALL meet this contract: `fledge changelog --tag v0.5.0` shows only that release

### REQ-changelog-004

The implementation SHALL meet this contract: `fledge changelog --unreleased` shows commits since the latest tag

### REQ-changelog-005

The implementation SHALL meet this contract: `fledge changelog --json` outputs structured JSON

### REQ-changelog-006

The implementation SHALL meet this contract: Merge commits are excluded

### REQ-changelog-007

The implementation SHALL meet this contract: Scoped commits (e.g., `fix(parser): msg`) are correctly parsed

### REQ-changelog-008

The implementation SHALL meet this contract: Non-conventional commits appear under "Other"
