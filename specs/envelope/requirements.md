---
spec: envelope.spec.md
---

## User Stories

- As a fledge developer, I want a single helper for each `--json` envelope dialect so I don't hand-roll `schema_version` at ~60 call sites and let shapes drift
- As a fledge developer, I want migrating a hand-rolled envelope to a helper to be a non-behavioral refactor, so I can adopt it without churning golden output
- As a fledge developer, I want the envelope dialect chosen explicitly (resource / action / flat) rather than copy-pasted from a neighboring command

## Acceptance Criteria

### REQ-envelope-001

The implementation SHALL meet this contract: `resource(schema_version, key, items)` builds `{schema_version, <key>: items}` with no `action` key

### REQ-envelope-002

The implementation SHALL meet this contract: `action(schema_version, action, fields)` builds `{schema_version, action, ...fields}`, inserting the fixed keys first then merging an object `fields`

### REQ-envelope-003

The implementation SHALL meet this contract: `versioned(schema_version, fields)` builds `{schema_version, ...fields}` with no `action` key

### REQ-envelope-004

The implementation SHALL meet this contract: Every envelope carries a `schema_version` from the caller-supplied `u32`

### REQ-envelope-005

The implementation SHALL meet this contract: Output is byte-for-byte identical to the equivalent hand-rolled `serde_json::json!`

### REQ-envelope-006

The implementation SHALL meet this contract: A non-object `fields` value contributes no extra keys; the envelope still carries its fixed keys

### REQ-envelope-007

The implementation SHALL meet this contract: Non-serializable `resource` items store `Value::Null` under the key instead of panicking

## Constraints

- Builders are infallible — they return a `serde_json::Value::Object` unconditionally
- Builders do not print and never mutate global state
- Depends only on `serde` (the `Serialize` bound) and `serde_json`

## Out of Scope

- Defining or validating per-command schema versions (each command owns its own constant)
- Serializing to a string / writing to stdout (callers do that)
- Envelope shapes beyond the three documented dialects
- Typed structs for individual command payloads
