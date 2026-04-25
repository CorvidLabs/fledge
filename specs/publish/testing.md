# Publish — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `create_repo_request_body` | Correct JSON payload for repo creation (`name`, `description`, `private`) |
| `create_repo_org_request` | Correct API path: `POST /orgs/<org>/repos` for org repos, `POST /user/repos` for personal |
| `set_repo_topic_additive` | New topic is merged into existing topic list, not replacing it |
| `check_repo_exists_404` | Returns `Ok(false)` on 404, not an error |
| `check_repo_exists_other_error` | Returns `Err` on non-200/404 responses |
| `urlencoded_repo_name` | Repo names with special characters are correctly encoded in API URLs |

## Integration Tests

The user-facing publish surfaces (`fledge templates publish`, `fledge lanes publish`, `fledge plugins publish`) drive these helpers and own their own e2e tests.

| Test | What it verifies |
|------|-----------------|
| `publish_live` | End-to-end publish via one of the surfaces (requires token, ignored by default) |
