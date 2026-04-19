---
spec: spec.spec.md
---

## Context

Fledge already uses spec-sync externally via a GitHub Action for CI validation. This module brings core spec validation into the fledge CLI itself, so developers can run `fledge spec check` locally before pushing.

The implementation focuses on structural validation (frontmatter, sections, file existence) rather than the full bidirectional export checking that the standalone spec-sync tool provides. This gives developers fast local feedback without requiring tree-sitter AST parsing.

## Related Modules

- `config` — reads `.specsync/config.toml` for spec settings
- `create_template` — similar scaffolding pattern used for `spec new`

## Design Decisions

- Parse YAML frontmatter manually (split on `---` delimiters) rather than adding a YAML dependency
- Reuse serde for frontmatter deserialization via a simple YAML-to-JSON converter
- Compatible with spec-sync v4 config format so CI and local checks stay in sync
