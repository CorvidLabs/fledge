---
spec: utils.spec.md
---

# Utils — Tasks

- [x] Implement process-wide non-interactive flag (`set`/`is`/`init_from_env`)
- [x] Add shared truthy-string parser and public `is_truthy_env` wrapper
- [x] Implement `is_interactive` and the `require_interactive` / `require_interactive_hint` guards
- [x] Implement kebab/camel/snake/pascal case converters
- [x] Implement `validate_project_name`, `validate_github_org`, `validate_commit_scope`
- [x] Implement `redact_secrets` with `LazyLock`-compiled `regex_lite` patterns
- [x] Cover all functions with in-module unit tests
- [x] Write utils spec and companion files

## Gaps

- Redaction covers only the four documented credential shapes; no entropy/heuristic token detection
- Case converters are ASCII-oriented; non-ASCII casing follows Rust's default `to_uppercase`/`to_lowercase` per character
