---
change: accept-windows-refused-connection-wording-in-the-isolation-test
artifact: testing
---

# Testing

## Automated

`tests/isolation.rs::default_temp_env_points_github_at_a_dead_port` is the only test in
scope. It runs in CI's `cargo test --verbose --locked` on `ubuntu-latest`,
`macos-latest` and `windows-latest`.

## What was verified

| Check | Result |
|---|---|
| `fledge run test -- --test isolation` on `main`'s lockfile (ureq 3.3.0), macOS | 4 passed |
| Same, on the v1.8.0 release lockfile (ureq 3.4.2), macOS | 4 passed |
| Spawned binary stderr, ureq 3.3.0, macOS | `io: Connection refused` (synthesized by ureq) |
| Spawned binary stderr, ureq 3.4.2, macOS | `io: Connection refused (os error 61)` (the OS's text) |
| Windows stderr, ureq 3.4.2 (#532 CI log) | contains `actively refused` and `os error 10061`, now accepted |
| `test` on all three OSes, this PR (ureq 3.3.0) | green, CI run 36257383427 |
| `test` on all three OSes, ureq 3.4.2 | green, CI run 36257571327 (`workflow_dispatch` probe: `main` + this fix + #532's release commit). The Windows log shows `Compiling ureq v3.4.2` and `default_temp_env_points_github_at_a_dead_port ... ok` |

## Acceptance signals

- `test (windows-latest)` is green on the rebased release PR, which carries ureq 3.4.2.
- `test (ubuntu-latest)`, `test (macos-latest)` and `test (windows-latest)` stay green on
  `main`'s ureq 3.3.0 lockfile.

## Rejection signals

- A run that reached api.github.com and failed there must still fail the test. The
  loopback assertion on `env.github_api_base()` is unchanged, and no accepted phrase
  (`connection refused`, `actively refused`, `os error 10061`) appears in a GitHub HTTP
  error or a DNS failure.
- A generic "the command failed" must not be enough. No new alternative matches
  `failed` or `error` alone.
