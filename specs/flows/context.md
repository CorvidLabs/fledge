---
spec: flows.spec.md
---

## Context

Flows extend the task runner into composable pipelines. While `fledge run` executes individual tasks, `fledge flow` chains them with ordering, parallelism, and failure control. This lets teams define CI-like workflows locally without external CI config.

## Related Modules

- `run` — task execution and project type detection
- `config` — GitHub token for search/import API calls
- `github` — GitHub API requests for search and clone
- `search` — response parsing shared with template search

## Design Decisions

- Flows share the same `fledge.toml` as tasks — no separate config file needed
- Parallel groups use threads rather than async — simpler for spawning external processes
- Community flows use GitHub topics (`fledge-flows`) following the same convention as templates
- Import merges flows/tasks without overwriting existing definitions
