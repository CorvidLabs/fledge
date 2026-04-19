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

### Integration Tests

- `github_api_get` against a public repo endpoint (requires network)
