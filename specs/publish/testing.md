# Publish — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `validate_valid_template` | Accepts directory with valid template.toml |
| `validate_missing_manifest` | Rejects directory without template.toml |
| `validate_invalid_manifest` | Rejects directory with unparseable template.toml |
| `validate_nonexistent_dir` | Rejects path that doesn't exist |
| `create_repo_request_body` | Correct JSON payload for repo creation |
| `create_repo_org_request` | Correct API path for org repos |
| `topics_include_fledge_template` | `fledge-template` always in topic list |
| `run_rejects_no_token` | Error when no GitHub token configured |

## Integration Tests

| Test | What it verifies |
|------|-----------------|
| `publish_live` | End-to-end publish to a test repo (requires token, ignored by default) |
