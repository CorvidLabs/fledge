---
spec: issues.spec.md
---

## Context

`fledge issues` brings issue triage into the terminal workflow. Developers can check open issues without switching to a browser, and the JSON mode supports scripting and automation. The module relies on the shared `github` module for API access and repo detection.

## Related Modules

- `github` — repo detection and authenticated API calls
- `config` — GitHub token storage
- `prs` — similar structure; issues view detects PRs and redirects

## Design Decisions

- Filter out pull requests client-side because the GitHub Issues API returns both issues and PRs
- Default to open state and sort by recently updated — matches the most common use case
- Detect PR numbers in `view` and suggest `fledge prs` rather than showing a confusing result
