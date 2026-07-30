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

- Covered indirectly by the `plugins`/`lanes`/`templates` `search` commands. A live call to `api.github.com` is a manual check only — no CI test may touch the network.
