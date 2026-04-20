---
module: metrics
type: requirements
---

# Metrics Requirements

## User Stories

1. As a developer, I want to see lines of code by language so I can understand my project's composition.
2. As a developer, I want to see file churn from git history so I can identify hotspots that change frequently.
3. As a developer, I want to see test file detection and test-to-code ratio so I can gauge test coverage at a glance.
4. As a developer, I want JSON output so I can pipe metrics data to other tools.

## Acceptance Criteria

- `fledge metrics` shows LOC by language with files, lines, code, blank, and comment counts
- `fledge metrics --churn` shows files sorted by commit frequency, filtered to existing files
- `fledge metrics --churn --limit 5` limits churn output to 5 entries
- `fledge metrics --tests` detects test files using language-specific patterns and reports ratio
- `fledge metrics --json` outputs structured JSON for all modes
- Directories like `.git`, `node_modules`, `target`, `vendor`, `dist`, `build` are excluded
- `--churn` outside a git repo returns an error message
