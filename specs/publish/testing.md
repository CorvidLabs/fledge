# Publish — Testing

## Unit Tests

All of these run against `test_support::MockHttpServer` (a loopback HTTP server) with the REST base redirected via `test_support::GithubBaseGuard`, and against local bare git repositories for the push path. No test contacts GitHub, and none reads the developer's git or fledge config.

| Test | What it verifies |
|------|-----------------|
| `get_authenticated_user_returns_login_and_sends_auth` | `GET /user` is issued with `Bearer`/`Accept`/`User-Agent` headers and the `login` field is returned |
| `get_authenticated_user_without_login_errors` | A 200 response with no `login` yields "Could not determine GitHub username" |
| `get_authenticated_user_on_unreachable_host_errors` | Connection refused surfaces as "GitHub API request failed" (no hang) |
| `check_repo_exists_true` | 200 → `Ok(true)`, with the token forwarded |
| `check_repo_exists_404` | Returns `Ok(false)` on 404, not an error |
| `check_repo_exists_other_error` | Returns `Err` on non-200/404 responses |
| `create_repo_request_body` | Correct JSON payload for repo creation (`name`, `description`, `private`, `auto_init`) and headers |
| `create_repo_org_request` | Correct API path: `POST /orgs/<org>/repos` for org repos, never `POST /user/repos` |
| `create_repo_422_reports_name_conflict` | 422 → "already exists", naming the repo |
| `create_repo_403_reports_missing_scope` | 403 → "'repo' scope" remediation |
| `set_repo_topic_additive` | New topic is merged into the existing topic list via `PUT`, not replacing it |
| `set_repo_topic_does_not_duplicate` | An already-present topic is not appended twice |
| `set_repo_topic_errors_when_fetch_fails` | A failed topic fetch is reported with context |
| `push_directory_initializes_commits_and_pushes` | First publish inits the repo, commits with the caller's message, pushes `main` to the remote, leaves no token in `.git/config`; a second publish takes the already-initialized/remote-set branch and adds a commit |
| `push_directory_surfaces_git_error` | A failing push reports "Failed to push to owner/repo" plus git's stderr |
| `run_publish_creates_repo_sets_topic_and_pushes` | Full orchestration on a missing repo: create → topic → push |
| `run_publish_skips_creation_when_repo_exists` | Existing repo: no creation call; `--json` implies consent so no prompt is reached |
| `run_publish_stops_when_repo_check_fails` | A failed existence check aborts before creating or pushing |
| `resolve_owner_prefers_org_without_calling_the_api` | `--org` short-circuits `GET /user`; `None` falls back to the authenticated login |
| `template_envelope_matches_legacy_inline_json`, `plugin_…`, `lanes_…` | The shared envelope is byte-identical to the three commands' former inline `json!` blocks, on the success and cancel paths |

## Integration Tests

The user-facing publish surfaces (`fledge templates publish`, `fledge lanes publish`, `fledge plugins publish`) drive these helpers and own their own e2e tests. A live publish against real GitHub is a manual step — there is no `#[ignore]`d placeholder for it, since the entire flow is now covered offline by the `run_publish` tests above.
