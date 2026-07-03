---
spec: meta.spec.md
---

# Meta — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `compute_file_hash_known_values` | Empty input and `b"hello world"` hash to their known SHA-256 digests |
| `compute_file_hash_is_deterministic` | Identical input yields identical output; digest is 64 chars long |
| `compute_file_hash_changes_with_content` | Different inputs (`b"a"` vs `b"b"`) produce different digests |

## Gaps

- No direct test of `write_project_meta` (directory/`.gitignore` creation, TOML output, hash map, skipping missing files) — currently exercised indirectly through template scaffolding
