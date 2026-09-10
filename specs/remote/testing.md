# Remote — Testing

## Unit Tests

- `is_remote_ref` accepts `owner/repo` and `owner/repo/sub` formats
- `is_remote_ref` rejects simple names, empty segments, and strings with spaces
- `parse_remote_ref` correctly splits owner, repo, and optional subpath
- `repo_url` generates correct URLs with and without tokens
- `cache_dir` returns platform-appropriate path ending in `fledge/templates`

## Integration Tests

- `tests/isolation.rs::templates_init_clones_a_remote_template_from_a_local_bare_repo` — the real clone path, offline: a `<owner>/<repo>.git` bare repo on disk stands in for github.com (`FLEDGE_TEST_GITHUB_REMOTE_BASE`, set by `TempEnv::with_github_remote_base`), and `templates init <owner>/<repo>` clones it, renders the template, and reports it in the `--json` envelope. Unix-only: the clone lands in `dirs::cache_dir()`, which follows `XDG_CACHE_HOME`/`HOME` on Linux and macOS but a known-folder API on Windows that no environment variable can redirect
- Clone and template discovery from a real GitHub repo (requires network, run manually)
