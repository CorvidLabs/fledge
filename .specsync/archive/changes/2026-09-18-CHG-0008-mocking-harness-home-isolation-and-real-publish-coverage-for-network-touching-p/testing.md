---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
artifact: testing
---

# Testing

## REQ-github-020: `github` resolves its REST base through `api_base()`

- Automated: `github::tests` exercise `github_api_get` against `MockHttpServer` via
  `GithubBaseGuard::api`, covering headers, percent-encoded query values, 404/403/generic
  status mapping, a non-JSON body, and an unreachable host.
- Code review: `api_base()` returns the production constant outside `cfg(test)`; the
  override is thread-local, so parallel tests cannot observe each other's redirection.

## REQ-publish-020: `publish` resolves the remote base through `remote_base()`

- Automated: `publish::tests::push_directory_initializes_commits_and_pushes` pushes to a
  local bare repository and asserts the resulting remote via `git remote get-url origin`
  with separators normalized on both sides — never by substring-matching an OS path against
  `.git/config` text, which git escapes and normalizes differently on Windows.
- Automated: the same test asserts no token appears in the `origin` URL, and a companion
  assertion confirms no token is written into `.git/config`.
- Automated: `run_publish` orchestration is covered for the create, skip-create-when-exists
  and abort-on-check-failure paths.

## REQ-llm-020: Ollama provider is exercisable against a loopback endpoint

- Automated: `llm::tests` assert the request body carries `model`, `prompt` and
  `stream: false`; that a configured key is sent as a Bearer header and no header is sent
  without one; and that HTTP status errors, undecodable bodies and connection refusal each
  map to a distinct, non-panicking error.

## REQ-doctor-020: reachability probe and isolated doctor CLI tests

- Automated: `probe_ollama_host` returns true against a live loopback mock and false
  against a closed port from `dead_port_url()`.
- Automated: `tests/doctor.rs` runs under `TempEnv`, which points `HOME`,
  `XDG_CONFIG_HOME` and `FLEDGE_CONFIG_DIR` at tempdirs, strips every provider API key
  plus `GITHUB_TOKEN`/`GH_TOKEN`, and aims `OLLAMA_HOST` at a closed port.

## Regression coverage

- `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are
  green; CI runs the suite on Linux, macOS and Windows and all three must pass.
- `src/publish.rs`'s previous `#[ignore]` stub is gone, replaced by real coverage of the
  authenticated-user, repo-existence, repo-creation, topic-set, push and orchestration paths.

## Rejection signals

- Any test contacting a non-loopback host, or reading the real `~/.config/fledge` or
  `~/.gitconfig`.
- A token appearing in `.git/config` or in the `origin` remote URL after a push.
- Any assertion that substring-matches an OS path against serialized file text — the defect
  that caused the Windows-only failure.
- Any release-build behavior change: `api_base()` and `remote_base()` must resolve to the
  production constants outside `cfg(test)`.
