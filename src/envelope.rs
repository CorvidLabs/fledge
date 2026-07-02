//! Shared constructors for fledge's two documented `--json` envelope dialects.
//!
//! Every `--json` output is `{schema_version, ...}` in one of two shapes (see
//! AGENTS.md § "Machine-readable surface"):
//!
//! - **resource dialect** — `{schema_version, <resource>: [...]}`, used by the
//!   pillar list/query commands (`plugins list`, `lanes search`, `templates
//!   list`, …). The resource key (`plugins`, `results`, `templates`) is the
//!   discriminator.
//! - **action dialect** — `{schema_version, action: "<verb>", ...}`, used by the
//!   cross-cutting commands (`doctor`, `run`, `ai`, `release`, …). The `action`
//!   string discriminates between commands sharing a similar shape.
//!
//! Building these by hand at ~60 call sites let shapes drift — the three
//! `search` commands shipped divergent `results[]` entries, for instance. These
//! helpers centralize the outer envelope so `schema_version` is never mistyped
//! and the dialect is chosen explicitly rather than copy-pasted.
//!
//! Provides both dialect builders: [`resource`] (pillar list/query commands)
//! and [`action`] (cross-cutting commands).

use serde::Serialize;
use serde_json::{Map, Value};

/// Build a **resource-dialect** envelope: `{schema_version, <resource_key>: items}`.
///
/// `items` is serialized in place — pass a `Vec<Value>`, a slice of any
/// `Serialize` type, or a pre-built `Value`.
///
/// ```ignore
/// let out = envelope::resource(PLUGINS_SEARCH_SCHEMA, "results", entries);
/// ```
pub fn resource(schema_version: u32, resource_key: &str, items: impl Serialize) -> Value {
    let mut map = Map::new();
    map.insert("schema_version".to_string(), Value::from(schema_version));
    map.insert(
        resource_key.to_string(),
        serde_json::to_value(items).unwrap_or(Value::Null),
    );
    Value::Object(map)
}

/// Build an **action-dialect** envelope: `{schema_version, action, ...fields}`.
///
/// `schema_version` and `action` are inserted first; the keys of `fields` are
/// merged in. `fields` must be a JSON object — any other `Value` contributes no
/// extra keys (the envelope still carries `schema_version` and `action`).
///
/// The serialized output is identical to a hand-rolled `json!({"schema_version":
/// …, "action": …, …fields})`: serde_json orders object keys the same way
/// regardless of insertion order, so migrating a call site to this helper is a
/// byte-for-byte-compatible refactor.
///
/// ```ignore
/// let out = envelope::action(WORK_START_SCHEMA, "work_start", serde_json::json!({
///     "branch": branch_name,
///     "base": base_branch,
/// }));
/// ```
pub fn action(schema_version: u32, action: &str, fields: Value) -> Value {
    let mut map = Map::new();
    map.insert("schema_version".to_string(), Value::from(schema_version));
    map.insert("action".to_string(), Value::from(action));
    if let Value::Object(obj) = fields {
        map.extend(obj);
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_wraps_items_under_key_with_schema_version() {
        let out = resource(3, "results", vec![serde_json::json!({"name": "a"})]);
        assert_eq!(out["schema_version"], 3);
        assert!(out.get("action").is_none());
        assert_eq!(out["results"][0]["name"], "a");
    }

    #[test]
    fn resource_accepts_empty_and_typed_items() {
        let empty = resource(1, "plugins", Vec::<Value>::new());
        assert_eq!(empty["plugins"], serde_json::json!([]));

        // A typed slice serializes the same as hand-rolled Values.
        let typed = resource(1, "results", ["x", "y"]);
        assert_eq!(typed["results"], serde_json::json!(["x", "y"]));
    }

    #[test]
    fn action_leads_with_schema_version_and_action_then_merges_fields() {
        let out = action(
            2,
            "release",
            serde_json::json!({ "dry_run": true, "version": "1.2.3" }),
        );
        assert_eq!(out["schema_version"], 2);
        assert_eq!(out["action"], "release");
        assert_eq!(out["dry_run"], true);
        assert_eq!(out["version"], "1.2.3");
    }

    #[test]
    fn action_is_byte_identical_to_hand_rolled_json() {
        // The migration guarantee: swapping a hand-rolled envelope for action()
        // must not change a single byte of serialized output.
        let built = action(
            1,
            "work_start",
            serde_json::json!({
                "branch": "t/feat/demo",
                "base": "main",
                "type": "feat",
                "prefix": Value::Null,
                "issue": Value::Null,
            }),
        );
        let hand = serde_json::json!({
            "schema_version": 1,
            "action": "work_start",
            "branch": "t/feat/demo",
            "base": "main",
            "type": "feat",
            "prefix": Value::Null,
            "issue": Value::Null,
        });
        assert_eq!(
            serde_json::to_string_pretty(&built).unwrap(),
            serde_json::to_string_pretty(&hand).unwrap()
        );
    }

    #[test]
    fn action_survives_non_object_fields() {
        let out = action(1, "noop", Value::Null);
        assert_eq!(out["schema_version"], 1);
        assert_eq!(out["action"], "noop");
        assert_eq!(out.as_object().unwrap().len(), 2);
    }
}
