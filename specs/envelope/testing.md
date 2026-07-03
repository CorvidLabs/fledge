---
spec: envelope.spec.md
---

# Envelope — Testing

## Unit Tests

| Test | What it verifies |
|------|-----------------|
| `resource_wraps_items_under_key_with_schema_version` | `resource` produces `{schema_version, <key>: items}` with no `action` key |
| `resource_accepts_empty_and_typed_items` | Empty vec serializes to `[]`; a typed slice serializes the same as hand-rolled Values |
| `action_leads_with_schema_version_and_action_then_merges_fields` | `action` inserts `schema_version` and `action`, then merges object fields |
| `action_is_byte_identical_to_hand_rolled_json` | `action` output equals the equivalent hand-rolled `json!` byte-for-byte |
| `action_survives_non_object_fields` | A non-object `fields` (e.g. `Null`) yields only `schema_version` + `action` |
| `versioned_prepends_schema_version_without_action` | `versioned` adds `schema_version` and merges fields, with no `action` key |
| `versioned_is_byte_identical_to_hand_rolled_json` | `versioned` output equals the equivalent hand-rolled `json!` byte-for-byte |
