---
spec: lanes.spec.md
---

## Context

Lanes extend the task runner into composable pipelines. While `fledge run` executes individual tasks, `fledge lanes` chains them with ordering, parallelism, and failure control. This lets teams define CI-like workflows locally without external CI config.

## Related Modules

- `run` — task execution and project type detection
- `config` — GitHub token for search/import API calls
- `github` — GitHub API requests for search and clone
- `search` — response parsing shared with template search

## Design Decisions

- Lanes share the same `fledge.toml` as tasks — no separate config file needed
- Parallel groups use threads rather than async — simpler for spawning external processes
- Community lanes use GitHub topics (`fledge-lanes`) following the same convention as templates
- Import merges lanes/tasks without overwriting existing definitions
