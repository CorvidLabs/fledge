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
//! This module currently provides the resource-dialect builder ([`resource`]),
//! adopted by the `*/search` commands. An action-dialect builder lands
//! alongside the first cross-cutting commands migrated to it.

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
}
