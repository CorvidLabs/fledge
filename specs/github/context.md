---
spec: github.spec.md
---

## Context

The `github` module is a shared foundation for all GitHub-facing features. Rather than each command re-implementing remote parsing and API calls, this module centralises that logic. It is used by `issues`, `prs`, and any future module that reads from the GitHub API.

## Related Modules

- `issues` — uses `detect_repo` and `github_api_get` to list/view issues
- `prs` — uses `detect_repo` and `github_api_get` to list/view pull requests
- `config` — stores the GitHub token
- `search` — provides `urlencod` for query parameter encoding

## Design Decisions

- Synchronous HTTP via `ureq` to avoid pulling in an async runtime — keeps the binary small
- Token lookup cascades: env var `FLEDGE_GITHUB_TOKEN` > `GITHUB_TOKEN` > config file — matches common CI and local setups
- `format_relative_time` uses `chrono` rather than manual math for correctness across DST boundaries
