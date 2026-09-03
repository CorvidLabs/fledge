---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
artifact: context
---

# Context

Issue #447 recorded three related test-quality gaps. Every network and subprocess path
in fledge was untested (LLM HTTP, GitHub API, remote fetch, plugin install, work git
flows); tests inherited the developer's real environment, so `doctor` probed whatever
endpoint the real config named and config tests could write a real
`~/.config/fledge/config.toml`; and `src/publish.rs` carried a tautological test
module whose only real case was an empty `#[ignore]` stub, leaving the GitHub publish
path at zero coverage.

PR #463 previously shipped the $HOME-isolation subset and auto-closed the issue
prematurely; it was reopened to track the remaining scope, namely a reusable mocking
harness and publish-flow isolation. This change delivers that remainder rather than
redoing #463's work.

The untested paths are exactly the ones where a regression is most expensive and least
likely to be caught by review: token handling, remote URL construction, and provider
error mapping. Leaving them uncovered also meant the test suite's correctness depended
on the machine it ran on.
