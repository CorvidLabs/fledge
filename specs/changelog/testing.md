---
module: changelog
type: testing
---

# Changelog Testing

## Unit Tests

| Test | Description |
|------|-------------|
| `classify_feat` | Parses `feat: message` correctly |
| `classify_fix_with_scope` | Parses `fix(scope): message` correctly |
| `classify_unknown` | Non-conventional commits grouped as "Other" |
| `classify_docs` | Parses `docs: message` correctly |
| `classify_all_prefixes` | All 10 conventional prefixes are recognized |
| `classify_no_space_after_colon` | Handles missing space after colon |

## Integration Tests

CLI-level integration tests are not included because changelog output depends on the git history of the repository under test, which varies.
