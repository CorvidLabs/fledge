---
spec: prs.spec.md
---

## Context

`fledge prs` complements `fledge work` by providing read access to pull requests. While `work pr` creates PRs, this module lets developers browse and inspect them. The detail view includes diff stats so developers can gauge PR size before reviewing.

## Related Modules

- `github` — repo detection and authenticated API calls
- `config` — GitHub token storage
- `work` — creates PRs; `prs` reads them
- `issues` — parallel structure for issues vs pull requests

## Design Decisions

- Separate icons for draft (open circle) and ready (filled circle) PRs — visual distinction without color dependency
- Merged state shown in magenta to match GitHub's purple merge color
- Diff stats fetched via a second API call only in detail view — avoids extra requests for listing
