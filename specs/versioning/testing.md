# Versioning — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `parse_valid_version` | Parses "1.2.3" correctly |
| `parse_with_v_prefix` | Strips leading "v" |
| `parse_invalid` | Rejects non-semver strings |
| `version_ordering` | 0.2.1 > 0.2.0, 1.0.0 > 0.99.99 |
| `check_compatible` | Returns Ok when current >= required |
| `check_incompatible` | Returns Err with upgrade message |
| `check_equal` | Returns Ok when versions are equal |
