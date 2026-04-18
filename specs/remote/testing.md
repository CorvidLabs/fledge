# Remote — Testing

## Unit Tests

- `is_remote_ref` accepts `owner/repo` and `owner/repo/sub` formats
- `is_remote_ref` rejects simple names, empty segments, and strings with spaces
- `parse_remote_ref` correctly splits owner, repo, and optional subpath
- `repo_url` generates correct URLs with and without tokens
- `cache_dir` returns platform-appropriate path ending in `fledge/templates`

## Integration Tests

- Clone and template discovery from a real GitHub repo (requires network, run manually)
