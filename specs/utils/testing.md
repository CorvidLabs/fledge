---
spec: utils.spec.md
---

# Utils — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `test_to_snake_case` / `_multiple_hyphens` / `_empty` / `_single_char` | `-`→`_`, lowercasing, empty and single-char edge cases |
| `test_to_pascal_case` / `_multiple_segments` / `_mixed_separators` / `_empty` / `_single_char` | Splits on `-`/`_`, capitalizes each segment |
| `test_to_kebab_case` / `_empty` | `_`→`-`, lowercasing, empty input |
| `test_to_camel_case` / `_multiple_segments` / `_empty` | PascalCase with a lowercased first char |
| `test_validate_project_name_valid` / `_empty` / `_path_traversal` / `_reserved` | Accepts normal names; rejects empty, `..`, `/`, and Windows device names |
| `test_validate_github_org_valid` / `_empty` / `_spaces_allowed` / `_slashes` | Accepts names (incl. spaces); rejects empty and slash-containing |
| `test_is_truthy_accepts_common_values` / `_rejects_common_falsy_values` | Truthy set is exactly `1`/`true`/`yes`/`y`/`on` (trimmed, case-insensitive) |
| `test_is_truthy_env_mirrors_is_truthy` | Public wrapper matches the private parser exactly |
| `test_set_and_is_non_interactive` | Flag set/read round-trips via the atomic |
| `test_require_interactive_bails_when_non_interactive_set` | Error names the flag/env var and offers an escape hatch |
| `test_is_interactive_respects_non_interactive_flag` | Flag forces non-interactive regardless of TTY |
| `validate_commit_scope_accepts_normal_scopes` | Accepts alnum/`-`/`_` scopes |
| `validate_commit_scope_rejects_injection` | Rejects whitespace, slashes, quotes, and prompt-injection payloads |
| `validate_commit_scope_empty` / `_caps_length` | Empty message text; 64-char boundary (64 ok, 65 err) |
| `redact_secrets_strips_authorization_header` | `Authorization:` value redacted to end-of-line |
| `redact_secrets_strips_x_access_token` | `x-access-token:` value redacted |
| `redact_secrets_strips_url_credentials` | `scheme://user:pass@host` → `scheme://[REDACTED]@host` |
| `redact_secrets_strips_bearer_token` | `Bearer <token>` redacted |
| `redact_secrets_passes_through_clean_input` | Clean input returned byte-identical |
| `redact_secrets_handles_case_insensitive_headers` | Header matching is case-insensitive |

## Notes

- Non-interactive tests share `crate::test_support::NonInteractiveGuard` to serialize on the process-wide atomic.
- Redaction fixtures use `FIXTURE_*_PLACEHOLDER` strings so they don't trip secret-scanning push protection.
