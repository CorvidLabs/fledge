---
module: remote
version: 2
status: active
files:
  - src/remote.rs

db_tables: []
depends_on: []
---

# Remote

## Purpose

Fetches templates from GitHub repositories. Clones repos to a local cache directory, supports authenticated access via GitHub token, and resolves subpaths within repos.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `cache_dir` | Returns the platform-appropriate cache directory for cloned template repos |
| `is_remote_ref` | Checks if a string looks like a GitHub `owner/repo` reference |
| `parse_remote_ref` | Splits a remote reference into owner, repo, and optional subpath |
| `fetch_repo` | Clones or updates a GitHub repo in the local cache |
| `clear_cache` | Removes a cached repo directory to force re-clone |
| `resolve_template_dir` | Fetches a repo and returns the path to the template directory |

### Structs & Enums

| Type | Description |
|------|-------------|

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `cache_dir` | `() -> PathBuf` | Returns `~/.cache/fledge/templates` (platform-aware) |
| `is_remote_ref` | `(&str) -> bool` | Returns true if string contains `/` with non-empty segments |
| `parse_remote_ref` | `(&str) -> (&str, &str, Option<&str>)` | Splits into (owner, repo, optional subpath) |
| `fetch_repo` | `(&str, &str, Option<&str>) -> Result<PathBuf>` | Clone or pull repo, returns local path |
| `clear_cache` | `(&str, &str) -> Result<()>` | Remove cached repo dir for owner/repo |
| `resolve_template_dir` | `(&str, &str, Option<&str>, Option<&str>) -> Result<PathBuf>` | Fetch repo and resolve optional subpath |

## Invariants

1. Repos are cached at `{cache_dir}/{owner}/{repo}`
2. Cached repos are updated with `git pull --ff-only` on subsequent access
3. If pull fails, falls back to fetch + reset
4. Token is embedded in HTTPS URL for authenticated access
5. `is_remote_ref` requires at least two non-empty `/`-separated segments
6. `parse_remote_ref` splits on first two `/` characters; everything after is the subpath

## Behavioral Examples

### Scenario: First-time clone

- **Given** `CorvidLabs/fledge-templates` has not been cached
- **When** `fetch_repo("CorvidLabs", "fledge-templates", None)` is called
- **Then** clones repo with `--depth 1` to cache dir and returns path

### Scenario: Cached repo update

- **Given** `CorvidLabs/fledge-templates` is already cached
- **When** `fetch_repo("CorvidLabs", "fledge-templates", None)` is called
- **Then** runs `git pull --ff-only` and returns existing cache path

### Scenario: Authenticated access

- **Given** a GitHub token is provided
- **When** `fetch_repo("CorvidLabs", "private-templates", Some("ghp_xxx"))` is called
- **Then** uses `https://ghp_xxx@github.com/CorvidLabs/private-templates.git` as clone URL

### Scenario: Subpath resolution

- **Given** `CorvidLabs/templates` is cached and contains `rust-cli/` subdirectory
- **When** `resolve_template_dir("CorvidLabs", "templates", Some("rust-cli"), None)` is called
- **Then** returns path to `{cache}/CorvidLabs/templates/rust-cli`

## Error Cases

| Condition | Behavior |
|-----------|----------|
| Repo doesn't exist or no access | Returns error with "Failed to clone" message |
| Subpath doesn't exist in repo | Returns error with "Subpath not found" message |
| Git not installed | Returns error from Command spawn |
| Pull fails and fetch also fails | Returns error with "Failed to update" message |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `dirs` | `cache_dir()` for platform cache path |
| `anyhow` | Error handling |

### Consumed By

| Module | What is used |
|--------|-------------|
| `init` | `is_remote_ref()`, `parse_remote_ref()`, `resolve_template_dir()`, `clear_cache()` |
| `templates` | `is_remote_ref()`, `parse_remote_ref()`, `resolve_template_dir()` |

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-04-18 | CorvidAgent | Initial spec |
