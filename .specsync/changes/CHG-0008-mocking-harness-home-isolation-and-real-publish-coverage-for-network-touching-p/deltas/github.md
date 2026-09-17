---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
module: github
---

## ADDED

### REQUIREMENT REQ-github-020

The `github` module SHALL resolve its REST base through `api_base()` rather than an
inline literal, so tests can redirect API traffic to a loopback server.

Acceptance Criteria
- `api_base()` returns the production `https://api.github.com` constant in non-test builds.
- Under `cfg(test)`, a thread-local override redirects the base without affecting other threads.
- `github_api_get` builds request URLs from `api_base()` and percent-encodes query values.
