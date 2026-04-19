---
module: search
version: 1
status: active
files:
  - src/search.rs

db_tables: []
depends_on:
  - specs/config/config.spec.md
---

# Search

## Purpose

Discovers fledge-compatible templates on GitHub by searching for repositories tagged with the `fledge-template` topic. Allows users to find community templates, view details, and use them directly with `fledge init`.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `SearchOptions` | Options struct for the search command |
| `SearchResult` | A single matching repository with metadata |
| `run` | Entry point that queries GitHub and displays matching templates |
| `full_name` | Method on `SearchResult` returning `owner/repo` string |
| `build_search_query` | Constructs GitHub search query string with `fledge-template` topic |
| `search_github` | Executes GitHub search API call and parses results |
| `parse_search_response` | Parses GitHub API JSON response into `Vec<SearchResult>` |
| `format_stars` | Formats star count with `k` suffix for thousands |
| `urlencod` | URL-encodes a string for use in query parameters |

### Structs & Enums

| Type | Description |
|------|-------------|
| `SearchOptions` | Command options: optional query string, limit, JSON output flag |
| `SearchResult` | A single matching repository with name, description, owner, stars, URL |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(SearchOptions) -> Result<()>` | Search GitHub API for fledge-template repos, display results |
| `full_name` | `(&self) -> String` | Returns `owner/repo` format string for a `SearchResult` |
| `build_search_query` | `(keyword: Option<&str>) -> String` | Constructs GitHub search query with `fledge-template` topic filter |
| `search_github` | `(keyword: Option<&str>, token: Option<&str>, limit: usize) -> Result<Vec<SearchResult>>` | Execute GitHub search API call and parse results |
| `parse_search_response` | `(body: &serde_json::Value) -> Result<Vec<SearchResult>>` | Parse GitHub API JSON into search results |
| `format_stars` | `(count: u64) -> String` | Format star count with `k` suffix for thousands |
| `urlencod` | `(s: &str) -> String` | URL-encode a string for use in query parameters |

## Invariants

1. Always searches for repos with the `fledge-template` topic
2. Optional keyword query narrows results within that topic
3. Results are sorted by stars (descending) by default
4. Works without authentication (lower rate limit) but respects `github.token` config
5. Limit defaults to 20 results
6. Each result includes owner/repo reference usable with `fledge init -t owner/repo`
7. JSON output mode (`--json`) prints machine-readable output

## Behavioral Examples

### Scenario: Basic search with no query

- **Given** GitHub has repos tagged `fledge-template`
- **When** `fledge search` is run
- **Then** displays up to 20 repos sorted by stars with name, description, owner, stars

### Scenario: Search with keyword filter

- **Given** GitHub has repos tagged `fledge-template`
- **When** `fledge search rust` is run
- **Then** displays repos matching both the topic and keyword "rust"

### Scenario: Custom limit

- **Given** user passes `--limit 5`
- **When** search executes
- **Then** displays at most 5 results

### Scenario: JSON output

- **Given** user passes `--json`
- **When** search executes
- **Then** outputs JSON array of result objects instead of formatted table

### Scenario: No results found

- **Given** no repos match the search criteria
- **When** search executes
- **Then** prints "No templates found." message

### Scenario: API rate limit exceeded

- **Given** GitHub returns 403 rate limit error
- **When** search executes
- **Then** returns friendly error suggesting `fledge config set github.token`

## Error Cases

| Condition | Behavior |
|-----------|----------|
| No internet / DNS failure | Returns error with connection context |
| GitHub API rate limit (403) | Returns error suggesting token configuration |
| GitHub API error (5xx) | Returns error with status code |
| Malformed API response | Returns parse error with context |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `ureq` | HTTP client for GitHub API |
| `serde_json` | JSON parsing of API responses |
| `console` | `style` for colored output |
| `anyhow` | Error handling |
| `config` | `Config::load()`, `github_token()` for authentication |

### Consumed By

| Module | What is used |
|--------|-------------|
| `main` | `run()`, `SearchOptions` for the `search` subcommand |

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-04-19 | CorvidAgent | Initial spec |
