---
change: CHG-0011-route-remote-template-cloning-through-the-shared-github-remote-base-helper
module: remote
---

## ADDED

### REQUIREMENT REQ-remote-010

Remote template cloning SHALL derive its git URL from the shared
`github::remote_url(owner, repo)` helper rather than an inline literal, so the remote
base can be redirected to a local bare repository under test.

Acceptance Criteria
- `clone_repo` composes its URL via `github::remote_url`, not an inline format string.
- With the remote base pointed at a local directory, `templates init <owner>/<repo>`
  clones and renders without network access.
- Reverting the redirection makes that test fail by attempting a real clone.
- Outside `cfg(debug_assertions)` the override is absent and the production constant is used.
