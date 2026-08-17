---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
module: publish
---

## ADDED

### REQUIREMENT REQ-publish-020

The `publish` module SHALL resolve the git remote base through `remote_base()` /
`remote_url(owner, repo)` rather than an inline literal, so the publish flow can be
exercised end to end against a local bare repository.

Acceptance Criteria
- `remote_url` composes the production `https://github.com/{owner}/{repo}.git` form in non-test builds.
- Under `cfg(test)`, a thread-local override redirects the remote base.
- A push never writes an authentication token into `.git/config` or the `origin` remote URL.
