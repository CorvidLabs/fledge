---
module: search
version: 3
status: active
files:
  - src/search.rs

db_tables: []
depends_on: []
---

# Search

## Purpose

Library helpers for GitHub topic-based discovery. Consumed by `templates search` (template discovery), `lanes search` (lane discovery), `plugins search` (plugin discovery), and `github_api_get` (URL encoding).

The split is intentional: discovering "GitHub repos tagged with topic X" is a generic mechanism, but each consumer wires it to a different topic and renders results differently. Keeping the mechanism here as a library and the user-facing surfaces in their respective callers means a future GitLab-search backend can swap this module out without touching the callers.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `SearchResult` | A single matching repository with metadata |
| `full_name` | Method on `SearchResult` returning `owner/repo` string |
| `build_search_query_ex` | Constructs a GitHub search query string with topic + optional keyword/author |
| `parse_search_response` | Parses GitHub API JSON response into `Vec<SearchResult>` |
| `format_stars` | Formats a star count with `k` suffix for thousands |
| `urlencod` | URL-encodes a string for use in query parameters |

### Structs & Enums

| Type | Description |
|------|-------------|
| `SearchResult` | A single matching repository: owner, name, description, stars, url, topics |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `full_name` | `(&self) -> String` | Returns `owner/repo` format string for a `SearchResult` |
| `build_search_query_ex` | `(keyword: Option<&str>, author: Option<&str>, topic: &str) -> String` | Constructs a GitHub search query string |
| `parse_search_response` | `(body: &serde_json::Value) -> Result<Vec<SearchResult>>` | Parses GitHub search API JSON |
| `format_stars` | `(count: u64) -> String` | Formats star count: `42`, `1.5k`, `123k` |
| `urlencod` | `(s: &str) -> String` | Percent-encodes a string for query parameters |

## Invariants

1. Callers compose the actual API call — this module only builds the query, encodes parameters, and parses responses
2. `build_search_query_ex` always emits the topic filter (`topic:<topic>`), placing it after any keyword and before any user filter so the resulting search string reads left-to-right
3. `parse_search_response` is tolerant: missing `description` becomes `"No description"`; missing `stargazers_count` becomes `0`; missing `topics` becomes an empty list. An item without an `owner.login` is skipped
4. `format_stars` uses `1.0k` formatting under 10k and `123k` (no decimal) above
5. `urlencod` keeps `[A-Za-z0-9-_.~]` unreserved per RFC 3986 and percent-encodes everything else; spaces become `%20` (not `+`)

## Behavioral Examples

### build_search_query_ex
```rust
build_search_query_ex(None, None, "fledge-template")
// "topic:fledge-template"

build_search_query_ex(Some("rust"), None, "fledge-template")
// "rust in:name,description,topics topic:fledge-template"

build_search_query_ex(Some("rust"), Some("CorvidLabs"), "fledge-template")
// "rust in:name,description,topics topic:fledge-template user:CorvidLabs"
```

### format_stars
```rust
format_stars(42)     // "42"
format_stars(1500)   // "1.5k"
format_stars(123456) // "123k"
```

### urlencod
```rust
urlencod("hello world")        // "hello%20world"
urlencod("topic:fledge-template")  // "topic%3Afledge-template"
```

## Error Cases

| Condition | Behavior |
|-----------|----------|
| Malformed API response (missing `items` array) | `parse_search_response` returns `Err` with context |
| Item missing `owner.login` | Item is silently skipped — keeps the rest of the response usable |

## Dependencies

| Crate/Module | What is used |
|-------------|-------------|
| `serde` / `serde_json` | `SearchResult` derive + JSON parsing of API responses |

## Consumed By

| Module | What is used |
|--------|-------------|
| `main` | `build_search_query_ex`, `parse_search_response`, `format_stars` for `fledge templates search` |
| `lanes` | `build_search_query_ex`, `parse_search_response`, `format_stars` for `fledge lanes search` |
| `plugin` | `build_search_query_ex` for `fledge plugins search` |
| `github` | `urlencod` for `github_api_get` query parameters |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-04-25 | `templates search` re-absorbed into core (`main.rs::search_templates`); the `fledge-plugin-templates-remote` plugin was redundant with the existing helpers and is dropped from `DEFAULT_PLUGINS`. Module remains a library of query/parse helpers consumed by `main`, `lanes`, `plugins`, and `github`. |
| 2 | 2026-04-25 | v0.15 tight-core: removed `run`, `SearchOptions`, and `search_github_ex` — the user-facing `templates search` command lived in `fledge-plugin-templates-remote` then. |
| 1 | 2026-04-19 | Initial spec |
