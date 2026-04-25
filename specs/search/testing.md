# Search — Testing

## Unit Tests

| Test | Description |
|------|-------------|
| `parse_search_response_valid` | Parses a valid GitHub search API JSON response into `Vec<SearchResult>` |
| `parse_search_response_empty` | Handles empty `items` array gracefully |
| `parse_search_response_missing_fields` | Defaults missing description / stars / topics to sensible values |
| `parse_search_response_skips_no_owner` | Items without `owner.login` are silently skipped |
| `build_search_query_no_keyword` | Builds correct query with only topic filter |
| `build_search_query_with_keyword` | Builds correct query combining topic and keyword |
| `build_search_query_with_author` | Adds `user:` filter when an author is provided |
| `format_stars_abbreviation` | Formats star counts (e.g. 1500 → "1.5k", 123456 → "123k") |
| `urlencod_unreserved` | Keeps `[A-Za-z0-9-_.~]` unencoded per RFC 3986 |
| `urlencod_special` | Percent-encodes spaces as `%20` (not `+`), colons as `%3A` |

## Integration Tests

The user-facing surfaces (`fledge templates search`, `fledge lanes search`, `fledge plugins search`) drive these helpers and are tested at the CLI level in `tests/templates.rs`, `tests/lanes.rs`, etc. Live network calls are gated behind `#[ignore]` — run with `cargo test -- --ignored`.

## Manual Testing

```bash
# Templates flavor (the user-facing search wired to these helpers)
fledge templates search                    # browse all fledge-template repos
fledge templates search rust               # filter by keyword
fledge templates search --author CorvidLabs
fledge templates search --limit 5 --json

# Lanes flavor
fledge lanes search

# Plugins flavor
fledge plugins search deploy
```
