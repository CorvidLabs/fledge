---
change: mask-recognized-secrets-in-config-set-confirmation
artifact: design
---

# Design

## Boundary

Preserve load -> set -> save -> confirmation. After save succeeds, select *** when Config::is_secret_key recognizes the key; otherwise display the unchanged supplied string. Only that display selection changes. Save/error behavior, schema, permissions and persistence values are retained.

Deprecated ai.claude.api_key stays classified because existing config still supports it. No operator config is used by the regression fixtures.

## Alternatives

Blanket suppression removes useful feedback and fails the visible non-secret control. Masking stored data breaks scalar persistence and fails exact marker assertions. A duplicated secret-key list risks divergence from Get; the existing classifier is sufficient.

## Security limits

This narrows successful-confirmation stdout exposure. It does not remove process argv/shell history, encrypt storage, audit unrelated commands, or qualify dependencies. The tests run the actual checkout binary with argument arrays and disposable homes/config/cwd; relevant credential variables are removed and all markers are synthetic. Child config operations have no individual timeout, so existing outer gate/CI timeout remains the bound. No network request or signing is required.
