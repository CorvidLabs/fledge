---
spec: github.spec.md
---

## Test Plan

### Unit Tests

- `detect_repo` with HTTPS URL (with and without `.git` suffix)
- `detect_repo` with SSH URL (`git@github.com:owner/repo.git`)
- `detect_repo` with token-authenticated HTTPS URL
- `detect_repo` with non-GitHub URL returns an error
- `format_relative_time` with timestamps seconds, minutes, hours, days ago
- `format_relative_time` with invalid input returns the raw string

- `build_api_url` with and without query params (encoding + `&` joining)
- `github_status_error_message` for 404 / 403 / uncategorized codes

### Endpoint Override Tests

The runtime (environment) override exists for integration tests, which drive a spawned binary the `cfg(test)` thread-local cannot reach. These tests pin down how little it is allowed to do:

- `endpoint_env_override_is_ignored_unless_set` — unset environment ⇒ `api_base`/`remote_base`/`remote_url` are the production constants
- `endpoint_env_override_redirects_only_in_debug_builds` — a loopback value redirects in a debug build and is ignored in a release build (the profile every shipped binary is built with)
- `endpoint_env_override_rejects_non_loopback_values` — `https://evil.example`, `http://evil.example`, the userinfo trick (`http://127.0.0.1@evil.example`), suffix lookalikes (`http://127.0.0.1.evil.example`) and scheme-relative values all fall back to `https://api.github.com`
- `remote_env_override_accepts_loopback_and_local_dirs` — loopback URLs (v4 and bracketed v6) and existing absolute directories are accepted; relative or non-existent paths and remote URLs are not

### Mocked HTTP Tests

`github_api_get` is exercised against `test_support::MockHttpServer` with the REST base redirected by `test_support::GithubBaseGuard` — the real request path, offline:

- `api_get_parses_body_and_sends_headers` — `Accept`, `User-Agent`, `Authorization: Bearer …`
- `api_get_without_token_sends_no_authorization` — anonymous requests carry no auth header
- `api_get_sends_encoded_query_params` — the query string reaching the server is percent-encoded
- `api_get_404_returns_the_remediation_message` / `api_get_403_returns_the_rate_limit_message` / `api_get_other_status_falls_back_to_generic_error`
- `api_get_non_json_body_errors_on_parse` — HTML/garbage body → "parsing GitHub API response"
- `api_get_unreachable_host_errors_without_hanging` — connection refused is an error, not a hang
- `search_results_flow_through_github_api_get` — a search payload decodes through `search::parse_search_response`

### Integration Tests

`tests/isolation.rs` drives the spawned binary, which is the only way to cover the environment override end to end:

- `github_api_base_override_routes_a_spawned_command_to_the_mock` — `templates search --json` under `TempEnv::with_github_api_base` returns the loopback mock's payload, and the mock records the `GET /search/repositories?…` it served
- `default_temp_env_points_github_at_a_dead_port` — with no explicit base, a `TempEnv`-wrapped search fails against a closed local port instead of reaching api.github.com
- `endpoint_override_is_absent_from_release_builds` — the debug-only gate is what `github_redirection_supported()` reports, so release runs skip rather than fall through to the network

A live call to `api.github.com` is a manual check only — no CI test may touch the network.
