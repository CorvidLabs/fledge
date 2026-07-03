---
spec: envelope.spec.md
---

# Envelope — Tasks

- [x] Write envelope spec
- [x] Implement `resource` (resource-dialect builder)
- [x] Implement `action` (action-dialect builder)
- [x] Implement `versioned` (flat versioned builder)
- [x] Cover all three builders with unit tests, including byte-for-byte parity
- [ ] Migrate remaining hand-rolled `--json` call sites to the helpers

## Gaps

- Call-site migration is incremental — not every `--json` command uses the helpers yet
- No compile-time enforcement that a command routes its envelope through this module
