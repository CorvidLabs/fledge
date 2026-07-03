---
spec: envelope.spec.md
---

## Key Decisions

- **Centralize the outer envelope, not the payloads.** Building `{schema_version, ...}` by hand at ~60 call sites let shapes drift — the three `search` commands once shipped divergent `results[]` entries. Three free functions own the outer shell; callers still build their own fields.
- **Three builders, one per documented dialect.** `resource` (pillar list/query), `action` (cross-cutting), and `versioned` (flat, neither dialect — e.g. `lanes run`/`lanes validate`).
- **Byte-for-byte compatibility.** serde_json orders object keys deterministically regardless of insertion order, so swapping a hand-rolled `json!` envelope for a helper cannot change a single byte of serialized output. This makes adoption a non-behavioral refactor.
- **Infallible by design.** No `Result`. `resource` falls back to `Value::Null` if items fail to serialize; `action`/`versioned` skip the merge when `fields` isn't an object. Callers never handle an envelope error.

## Files to Read First

- `src/envelope.rs` — the whole module (three functions plus unit tests)
- `CLAUDE.md` / AGENTS.md § "Machine-readable surface" — the envelope contract these builders implement

## Current Status

Active, version 1. All three builders implemented and unit-tested. Call-site migration to the helpers is incremental.

## Notes

- `action` and `versioned` merge `fields` only when it is a `Value::Object`; any other `Value` contributes no keys.
- The helpers do not know or check schema-version values — each command passes its own constant.
