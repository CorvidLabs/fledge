---
module: publish
version: 5
status: active
files:
  - src/publish.rs

db_tables: []
depends_on:
  - config
---

# Publish

## Purpose

Shared GitHub-publishing helpers used by `templates publish`, `lanes publish`, and `plugins publish`: authenticate to GitHub, create or check a repo, set topics, and push a directory. The module provides both low-level helpers (`get_authenticated_user`, `check_repo_exists`, `create_github_repo`, `set_repo_topic`, `push_directory`) and a shared orchestration (`publish_preflight` → `resolve_owner` → `run_publish`) so the three publish commands no longer duplicate the check-or-create / confirm / topic / push / envelope flow. Each caller still owns the artifact-specific concerns (manifest validation, repo-name derivation, the `--json` envelope's artifact fields) and passes them in via [`PublishRequest`]; the shared code owns the parts that must never drift between commands (token/path errors, the confirmation prompt, the hardened push, the envelope skeleton).

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `get_authenticated_user` | Fetches the GitHub username for the configured token |
| `check_repo_exists` | Checks whether a repo already exists on GitHub |
| `create_github_repo` | Creates a new GitHub repository via the API |
| `set_repo_topic` | Sets a single topic on a GitHub repository |
| `push_directory` | Initializes git (if needed) and pushes directory contents to GitHub with a caller-supplied commit message |
| `run_git` | Runs a git command in a given directory |
| `publish_preflight` | Shared head of every publish flow: load config, require a GitHub token, canonicalize the path |
| `resolve_owner` | Resolves the repo owner — `--org` if given, else the authenticated user |
| `PublishRequest` | Struct carrying the per-artifact differences (topic, commit message, envelope fields, noun/verb/command) into `run_publish` |
| `run_publish` | Shared orchestration tail: check-or-create the repo, confirm, set topic, push, and emit the envelope or success text |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `get_authenticated_user` | `(token: &str) -> Result<String>` | Returns the authenticated GitHub login |
| `check_repo_exists` | `(owner, repo, token: &str) -> Result<bool>` | True if repo exists; false on 404; bails on other errors |
| `create_github_repo` | `(name, description, private, org, token: &str) -> Result<()>` | Creates a repo under the user or an organization |
| `set_repo_topic` | `(owner, repo, topic, token: &str) -> Result<()>` | Adds a single topic to the existing topic set |
| `push_directory` | `(path, owner, repo, token, commit_message: &str, json: bool) -> Result<()>` | Force-pushes the directory's tracked content to `origin/main` using token-based auth; commits with `commit_message`; suppresses the progress line when `json` is set |
| `run_git` | `(dir, args: &[&str]) -> Result<()>` | Runs git in the given directory; suppresses stdout/stderr |
| `publish_preflight` | `(path: &Path) -> Result<(String, PathBuf)>` | Returns `(token, canonical_path)`; bails with the shared "No GitHub token configured" / "Directory not found" messages |
| `resolve_owner` | `(org: Option<&str>, token: &str) -> Result<String>` | `--org` value if present, else `get_authenticated_user` (only then is `GET /user` hit) |
| `run_publish` | `(req: PublishRequest) -> Result<()>` | Drives check-or-create → confirm → set topic → push → envelope/text for all three commands |

## Invariants

1. A GitHub token with `repo` scope must be passed to every helper that talks to the API; callers are responsible for resolving the token from config or env
2. `create_github_repo` returns a clear error message for the common failure modes — 422 (name conflict / invalid name), 403 (insufficient scope) — so callers don't have to interpret raw HTTP codes
3. `push_directory` uses an in-memory `http.extraheader` env-injection trick to avoid embedding the token in the persisted git remote; the remote is reset to a clean URL after the push. The commit subject is caller-supplied (`commit_message`) so each command records its own — a template, plugin, and lane publish no longer all commit "Publish fledge template". On push failure git's stderr is surfaced through `crate::utils::redact_secrets` so the injected token never leaks
4. `set_repo_topic` is additive — it merges the new topic into the existing topic set rather than replacing the whole list
5. Caller modules (`templates publish`, `lanes publish`, `plugins publish`) are responsible for the artifact-specific concerns: validating template/lane/plugin manifests, deriving the repo name/description, and supplying the `--json` envelope's artifact and hint fields via `PublishRequest`
6. `run_publish` is the single source of the check-or-create / confirm / topic / push / envelope flow. It emits no non-JSON text to stdout when `req.json` is set (the envelope is the only stdout), and it assembles the envelope via `crate::envelope::action` so the shared `schema_version`/`action`/`cancelled`/`repo`/`topic` keys stay byte-identical across the three commands regardless of insertion order

## Behavioral Examples

### create_github_repo — under an organization
```rust
create_github_repo("my-template", "A new template", false, Some("CorvidLabs"), token)?;
// POST https://api.github.com/orgs/CorvidLabs/repos
```

### push_directory — token-based auth without persisting credentials
```rust
push_directory(&path, "CorvidLabs", "my-plugin", token, "Publish fledge plugin", json)?;
// `git init` if needed, `git add -A`, `git commit -m "Publish fledge plugin"`,
// then a force-push to https://github.com/CorvidLabs/my-plugin.git using a
// one-shot `http.extraheader` injection so the token never lands in
// .git/config. The "Force-pushing…" progress line is suppressed when `json`.
```

### run_publish — shared orchestration driven by a caller-built request
```rust
let mut extra_fields = serde_json::Map::new();
extra_fields.insert("plugin".into(), json!({ "name": name, "version": version, "description": desc }));
extra_fields.insert("install_hint".into(), json!(format!("fledge plugins install {owner}/{repo}")));
run_publish(PublishRequest {
    path: &path, owner: &owner, repo_name: &repo, description: desc,
    private, org, token: &token, yes, json,
    topic: "fledge-plugin", commit_message: "Publish fledge plugin", noun: "plugin",
    schema_version: PLUGINS_PUBLISH_SCHEMA, success_verb: "Install",
    success_command: &format!("fledge plugins install {owner}/{repo}"), extra_fields,
})?;
// Checks the repo, prompts (unless --yes/--json), creates it if missing,
// sets the topic, pushes, and prints the {schema_version, action:"publish", …}
// envelope — identical control flow for templates/plugins/lanes.
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
| 5 | 2026-07-03 | Deduplicated the triplicated publish orchestration (#443): added `publish_preflight`, `resolve_owner`, `PublishRequest`, and `run_publish`; the three publish commands now share one check/confirm/create/topic/push/envelope flow. `push_directory` gained `commit_message` (fixes plugin/lane commits mislabeled "Publish fledge template") and a `json` flag (suppresses the "Force-pushing…" line so `--json` stdout is a single envelope). |
| 4 | 2026-04-25 | `templates publish` re-absorbed into core (`main.rs::publish_template`); `fledge-plugin-templates-remote` was duplicating these helpers in shell and is dropped from `DEFAULT_PLUGINS`. Module remains a library of helpers consumed by `templates publish`, `lanes publish`, and `plugins publish`. |
| 3 | 2026-04-25 | v0.15 tight-core: removed the `run` / `PublishOptions` / `validate_template` / `set_repo_topics` exports. The user-facing `templates publish` command lived in `fledge-plugin-templates-remote` then. |
| 2 | 2026-04-22 | Updated exports for plugin/lane publish support; document newly-public helpers |
| 1 | 2026-04-19 | Initial spec |
