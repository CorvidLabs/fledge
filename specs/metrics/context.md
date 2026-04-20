---
spec: metrics.spec.md
---

## Context

`fledge metrics` gives developers quick project health signals without installing separate tools like `cloc` or `tokei`. It focuses on three dimensions: code volume (LOC), change hotspots (churn), and test coverage (ratio). These are useful for assessing unfamiliar codebases and tracking project growth.

## Related Modules

- `run` — provides `detect_project_type` for language-aware defaults

## Design Decisions

- Built-in rather than shelling out to `cloc`/`tokei` — keeps fledge dependency-free for basic metrics
- Churn uses `git log --name-only` for simplicity — no git2 dependency
- Test detection is pattern-based per language rather than running test frameworks
- Excluded directories are hardcoded — covers all major ecosystems
