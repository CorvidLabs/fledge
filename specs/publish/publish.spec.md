---
module: publish
version: 3
status: active
files:
  - src/publish.rs

db_tables: []
depends_on:
  - config
---

# Publish

## Purpose

Shared GitHub-publishing helpers used by `lanes publish` and `plugins publish`: authenticate to GitHub, create or check a repo, set topics, and push a directory. As of v0.15 this module is a *library* — the user-facing `templates publish` command moved out to `fledge-plugin-templates-remote`, so the publish module no longer exposes a `run`/`PublishOptions` entry point or `validate_template`. Other modules consume the lower-level helpers directly.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `get_authenticated_user` | Fetches the GitHub username for the configured token |
| `check_repo_exists` | Checks whether a repo already exists on GitHub |
| `create_github_repo` | Creates a new GitHub repository via the API |
| `set_repo_topic` | Sets a single topic on a GitHub repository |
| `push_directory` | Initializes git (if needed) and pushes directory contents to GitHub |
| `run_git` | Runs a git command in a given directory |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `get_authenticated_user` | `(token: &str) -> Result<String>` | Returns the authenticated GitHub login |
| `check_repo_exists` | `(owner, repo, token: &str) -> Result<bool>` | True if repo exists; false on 404; bails on other errors |
| `create_github_repo` | `(name, description, private, org, token: &str) -> Result<()>` | Creates a repo under the user or an organization |
| `set_repo_topic` | `(owner, repo, topic, token: &str) -> Result<()>` | Adds a single topic to the existing topic set |
| `push_directory` | `(path, owner, repo, token: &str) -> Result<()>` | Force-pushes the directory's tracked content to `origin/main` using token-based auth |
| `run_git` | `(dir, args: &[&str]) -> Result<()>` | Runs git in the given directory; suppresses stdout/stderr |

## Invariants

1. A GitHub token with `repo` scope must be passed to every helper that talks to the API; callers are responsible for resolving the token from config or env
2. `create_github_repo` returns a clear error message for the common failure modes — 422 (name conflict / invalid name), 403 (insufficient scope) — so callers don't have to interpret raw HTTP codes
3. `push_directory` uses an in-memory `http.extraheader` env-injection trick to avoid embedding the token in the persisted git remote; the remote is reset to a clean URL after the push
4. `set_repo_topic` is additive — it merges the new topic into the existing topic set rather than replacing the whole list
5. Caller modules (`lanes publish`, `plugins publish`, `fledge-plugin-templates-remote`) are responsible for the user-facing concerns: validating template/lane/plugin manifests, prompting for confirmation, formatting output

## Behavioral Examples

### create_github_repo — under an organization
```rust
create_github_repo("my-template", "A new template", false, Some("CorvidLabs"), token)?;
// POST https://api.github.com/orgs/CorvidLabs/repos
```

### push_directory — token-based auth without persisting credentials
```rust
push_directory(&path, "CorvidLabs", "my-template", token)?;
// `git init` if needed, `git add -A`, `git commit`, then a force-push
// to https://github.com/CorvidLabs/my-template.git using a one-shot
// `http.extraheader` injection so the token never lands in .git/config.
```

### set_repo_topic — additive
```rust
// Existing topics: ["rust", "cli"]
set_repo_topic("CorvidLabs", "my-template", "fledge-template", token)?;
// Resulting topics: ["rust", "cli", "fledge-template"]
```

## Error Cases

| Error | Condition |
|-------|-----------|
| 422 from create_github_repo | Repo name already exists or is invalid |
| 403 from create_github_repo | Token lacks `repo` scope |
| Git push failed | Auth issue, network, or branch protection |
| run_git non-zero | The git command returned a non-zero exit |

## Dependencies

| Crate/Module | What is used |
|-------------|-------------|
| `ureq` | HTTP client for GitHub API |
| `serde_json` | JSON construction and parsing for API calls |
| `base64` | Encode `x-access-token:<token>` for HTTP Basic auth |
| `anyhow` | Error handling |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-04-25 | v0.15 tight-core: removed the `run` / `PublishOptions` / `validate_template` / `set_repo_topics` exports. The user-facing `templates publish` command now lives in `fledge-plugin-templates-remote`; this module is a library of shared helpers for the remaining in-core publish callers (`lanes publish`, `plugins publish`). |
| 2 | 2026-04-22 | Updated exports for plugin/lane publish support; document newly-public helpers |
| 1 | 2026-04-19 | Initial spec |
