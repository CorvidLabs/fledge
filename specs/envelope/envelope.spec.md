---
module: envelope
version: 1
status: active
files:
  - src/envelope.rs

db_tables: []
depends_on: []
---

# Envelope

## Purpose

Shared constructors for fledge's `--json` output envelopes. Every `--json` command emits `{schema_version, ...}` in one of the documented dialects, and building those envelopes by hand at ~60 call sites let their shapes drift (the three `search` commands once shipped divergent `results[]` entries). This module centralizes the outer envelope so `schema_version` is never mistyped and the dialect is chosen explicitly rather than copy-pasted.

It provides three builders covering the documented envelope shapes:

- **resource dialect** — `{schema_version, <resource_key>: items}`, used by the pillar list/query commands (`plugins list`, `lanes search`, `templates list`, …). The resource key (`plugins`, `results`, `templates`) is the discriminator.
- **action dialect** — `{schema_version, action: "<verb>", ...fields}`, used by the cross-cutting commands (`doctor`, `run`, `ai`, `release`, …). The `action` string discriminates between commands sharing a similar shape.
- **flat versioned** — `{schema_version, ...fields}`, for the handful of commands whose shape is neither dialect (e.g. `lanes run`, `lanes validate`) — named fields with no `action` key and no single resource array.

Each builder produces output byte-for-byte identical to the equivalent hand-rolled `serde_json::json!`, so migrating a call site to a helper is a non-behavioral refactor.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `resource` | Build a resource-dialect envelope `{schema_version, <resource_key>: items}`; serializes any `Serialize` items in place under the given key |
| `action` | Build an action-dialect envelope `{schema_version, action, ...fields}`; inserts `schema_version` and `action`, then merges the keys of the `fields` object |
| `versioned` | Build a flat versioned envelope `{schema_version, ...fields}` — like `action` but without the `action` key |

### Structs & Enums

| Type | Description |
|------|-------------|
| (none) | This module exports only free functions; it defines no public structs or enums |

## Invariants

1. Every envelope carries a `schema_version` key set from the caller-supplied `u32`, inserted before any other key.
2. `resource` serializes `items` via `serde_json::to_value`; a serialization failure yields `Value::Null` under the resource key rather than panicking or dropping the key.
3. `action` always inserts both `schema_version` and `action`, then merges the keys of `fields` when `fields` is a JSON object.
4. `versioned` inserts `schema_version` and merges the keys of `fields` when `fields` is a JSON object; it never adds an `action` key.
5. For `action` and `versioned`, a non-object `fields` value (e.g. `Value::Null`) contributes no extra keys — the envelope still carries its fixed keys (`schema_version`, and `action` for `action`).
6. Output is byte-for-byte identical to the equivalent hand-rolled `serde_json::json!({...})`: because serde_json orders object keys deterministically regardless of insertion order, migrating a call site to a helper cannot change a single byte of serialized output.
7. Builders return an owned `serde_json::Value::Object`; they do not print, and they never mutate global state.

## Behavioral Examples

### Scenario: Resource-dialect envelope wraps items under a discriminator key
```
Given a caller invokes resource(3, "results", vec![{"name": "a"}])
Then the output is {"schema_version": 3, "results": [{"name": "a"}]}
And there is no "action" key
```

### Scenario: Action-dialect envelope leads with schema_version and action, then merges fields
```
Given a caller invokes action(2, "release", {"dry_run": true, "version": "1.2.3"})
Then the output is {"schema_version": 2, "action": "release", "dry_run": true, "version": "1.2.3"}
And it is byte-for-byte identical to the equivalent hand-rolled json! macro
```

### Scenario: Flat versioned envelope adds schema_version without an action key
```
Given a caller invokes versioned(1, {"lane": "ci", "success": true})
Then the output is {"schema_version": 1, "lane": "ci", "success": true}
And there is no "action" key
```

## Error Cases

| Error | Condition |
|-------|-----------|
| (no fallible path) | All three builders are infallible and return a `Value` unconditionally |
| Non-serializable resource items | `resource` stores `Value::Null` under the resource key instead of failing |
| Non-object `fields` | `action`/`versioned` skip the merge and return only their fixed keys |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `serde` | `Serialize` bound on `resource`'s `items` argument |
| `serde_json` | `Value`, `Map`, and `to_value` for building and serializing envelopes |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-07-03 | Initial spec |
