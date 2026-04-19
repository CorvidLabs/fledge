# Search — Testing

## Unit Tests

| Test | Description |
|------|-------------|
| `parse_search_response_valid` | Parses a valid GitHub search API JSON response into `Vec<SearchResult>` |
| `parse_search_response_empty` | Handles empty `items` array gracefully |
| `parse_search_response_missing_fields` | Handles repos with null description |
| `build_search_query_no_keyword` | Builds correct query with only topic filter |
| `build_search_query_with_keyword` | Builds correct query combining topic and keyword |
| `format_stars_abbreviation` | Formats star counts (e.g. 1500 -> "1.5k") |
| `search_result_usage_hint` | Generates correct `fledge init -t owner/repo` hint |
| `json_output_format` | Serializes results to expected JSON structure |

## Integration Tests

Integration tests require network access and are gated behind `#[ignore]` — run with `cargo test -- --ignored`.

| Test | Description |
|------|-------------|
| `live_search_returns_results` | Performs actual GitHub API search (ignored by default) |

## Manual Testing

```bash
# Basic search
fledge search

# Search with keyword
fledge search rust

# JSON output
fledge search --json

# With limit
fledge search --limit 5

# Combined
fledge search python --limit 3 --json
```
