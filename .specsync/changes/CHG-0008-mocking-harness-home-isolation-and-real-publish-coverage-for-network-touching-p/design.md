---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
artifact: design
---

# Design

## Harness

A dependency-free `MockHttpServer` in `src/test_support.rs` binds `127.0.0.1:0`,
serves thread-per-connection, and shuts down on drop. Tests register responses with
`server.on(method, path, MockResponse::json(..))` and assert against recorded
`RecordedRequest` values (method, path, query, headers, body).

`wiremock` was deliberately rejected: it would introduce tokio, while all fledge HTTP
is blocking `ureq`. The hand-rolled server adds no dependency.

Supporting guards, all RAII and thread-local so parallel tests cannot observe each
other's overrides:

- `GithubBaseGuard::api` / `::api_and_remote` — redirect the GitHub REST base and the
  git remote base.
- `GitIdentityGuard` — pins git identity and sets `GIT_CONFIG_GLOBAL` /
  `GIT_CONFIG_NOSYSTEM` so commit-producing code never reads the real `~/.gitconfig`.
- `dead_port_url()` — a closed loopback port for connection-refused paths.

For integration tests, `TempEnv` in `tests/common/mod.rs` spawns `fledge` with a
fresh `HOME`, `XDG_CONFIG_HOME` and `FLEDGE_CONFIG_DIR`, `FLEDGE_NON_INTERACTIVE=1`,
every provider API key plus `GITHUB_TOKEN`/`GH_TOKEN` removed, and `OLLAMA_HOST`
pointed at a closed port.

## Production seams

Two inline constants become functions with `#[cfg(test)]`-only thread-local overrides:
`github::api_base()` and `publish::remote_base()` / `remote_url(owner, repo)`. Release
builds are byte-identical and continue to use the production constants; the seams exist
solely so `run_publish` can be exercised end to end against a local bare repository.

## Cross-platform assertion rule

Assertions must never substring-match an OS path against serialized file text. Git
escapes `\\` when writing config values and may normalize separators in remote URLs, so
such an assertion is guaranteed to fail on Windows. Remote URLs are asserted via
`git remote get-url origin` with both sides separator-normalized.
