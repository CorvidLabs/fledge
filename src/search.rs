use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub owner: String,
    pub name: String,
    pub description: String,
    pub stars: u64,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

impl SearchResult {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Canonical `--json` entry shared by every `*/search` command, so their
    /// `results[]` items have an identical shape. Historically these drifted:
    /// `plugins search` emitted `full_name` (no `owner`) while `templates
    /// search` / `lanes search` emitted `owner` (no `full_name`). This superset
    /// carries both — plus `topics` (always, `[]` when empty) and the
    /// caller-supplied trust tier — so no consumer of any of the three breaks
    /// and the shapes can no longer diverge.
    pub fn to_json(&self, trust_tier: &str) -> serde_json::Value {
        serde_json::json!({
            "owner": self.owner,
            "name": self.name,
            "full_name": self.full_name(),
            "description": self.description,
            "stars": self.stars,
            "url": self.url,
            "topics": self.topics,
            "trust_tier": trust_tier,
        })
    }
}

pub fn build_search_query_ex(keyword: Option<&str>, author: Option<&str>, topic: &str) -> String {
    build_search_query(keyword, author, topic, None)
}

pub fn build_search_query(
    keyword: Option<&str>,
    author: Option<&str>,
    topic: &str,
    extra_topic: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(kw) = keyword {
        parts.push(format!("{kw} in:name,description,topics"));
    }
    parts.push(format!("topic:{topic}"));
    if let Some(et) = extra_topic {
        parts.push(format!("topic:{et}"));
    }
    if let Some(a) = author {
        parts.push(format!("user:{a}"));
    }
    parts.join(" ")
}

pub fn parse_search_response(body: &serde_json::Value) -> Result<Vec<SearchResult>> {
    let items = body
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("unexpected GitHub API response: missing 'items' array"))?;

    let results: Vec<SearchResult> = items
        .iter()
        .filter_map(|item| {
            let owner = item
                .get("owner")
                .and_then(|o| o.get("login"))
                .and_then(|l| l.as_str())?;
            let name = item.get("name").and_then(|n| n.as_str())?;
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("No description");
            let stars = item
                .get("stargazers_count")
                .and_then(|s| s.as_u64())
                .unwrap_or(0);
            let url = item.get("html_url").and_then(|u| u.as_str()).unwrap_or("");
            let topics = item
                .get("topics")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Some(SearchResult {
                owner: owner.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                stars,
                url: url.to_string(),
                topics,
            })
        })
        .collect();

    Ok(results)
}

pub fn format_stars(count: u64) -> String {
    if count >= 1000 {
        let k = count as f64 / 1000.0;
        if k >= 10.0 {
            format!("{:.0}k", k)
        } else {
            format!("{:.1}k", k)
        }
    } else {
        format!("{}", count)
    }
}

pub fn urlencod(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            b' ' => "%20".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_response_valid() {
        let json = serde_json::json!({
            "total_count": 2,
            "items": [
                {
                    "name": "fledge-rust-template",
                    "owner": { "login": "CorvidLabs" },
                    "description": "A Rust CLI template for fledge",
                    "stargazers_count": 42,
                    "html_url": "https://github.com/CorvidLabs/fledge-rust-template"
                },
                {
                    "name": "fledge-python",
                    "owner": { "login": "someuser" },
                    "description": "Python project template",
                    "stargazers_count": 10,
                    "html_url": "https://github.com/someuser/fledge-python"
                }
            ]
        });

        let results = parse_search_response(&json).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].owner, "CorvidLabs");
        assert_eq!(results[0].name, "fledge-rust-template");
        assert_eq!(results[0].stars, 42);
        assert_eq!(results[1].full_name(), "someuser/fledge-python");
    }

    #[test]
    fn parse_search_response_empty() {
        let json = serde_json::json!({
            "total_count": 0,
            "items": []
        });

        let results = parse_search_response(&json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn parse_search_response_missing_description() {
        let json = serde_json::json!({
            "total_count": 1,
            "items": [
                {
                    "name": "bare-template",
                    "owner": { "login": "user" },
                    "description": null,
                    "stargazers_count": 5,
                    "html_url": "https://github.com/user/bare-template"
                }
            ]
        });

        let results = parse_search_response(&json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].description, "No description");
    }

    #[test]
    fn parse_search_response_missing_items() {
        let json = serde_json::json!({ "total_count": 0 });
        let result = parse_search_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn build_search_query_no_keyword() {
        let q = build_search_query_ex(None, None, "fledge-template");
        assert_eq!(q, "topic:fledge-template");
    }

    #[test]
    fn build_search_query_with_keyword() {
        let q = build_search_query_ex(Some("rust"), None, "fledge-template");
        assert_eq!(q, "rust in:name,description,topics topic:fledge-template");
    }

    #[test]
    fn build_search_query_ex_with_author() {
        let q = build_search_query_ex(Some("rust"), Some("corvidlabs"), "fledge-template");
        assert_eq!(
            q,
            "rust in:name,description,topics topic:fledge-template user:corvidlabs"
        );
    }

    #[test]
    fn build_search_query_ex_author_only() {
        let q = build_search_query_ex(None, Some("corvidlabs"), "fledge-plugin");
        assert_eq!(q, "topic:fledge-plugin user:corvidlabs");
    }

    #[test]
    fn format_stars_below_thousand() {
        assert_eq!(format_stars(0), "0");
        assert_eq!(format_stars(42), "42");
        assert_eq!(format_stars(999), "999");
    }

    #[test]
    fn format_stars_thousands() {
        assert_eq!(format_stars(1000), "1.0k");
        assert_eq!(format_stars(1500), "1.5k");
        assert_eq!(format_stars(2300), "2.3k");
    }

    #[test]
    fn format_stars_ten_thousands() {
        assert_eq!(format_stars(10000), "10k");
        assert_eq!(format_stars(15000), "15k");
        assert_eq!(format_stars(123456), "123k");
    }

    #[test]
    fn search_result_full_name() {
        let r = SearchResult {
            owner: "CorvidLabs".to_string(),
            name: "fledge-templates".to_string(),
            description: "Templates".to_string(),
            stars: 10,
            url: "https://github.com/CorvidLabs/fledge-templates".to_string(),
            topics: vec![],
        };
        assert_eq!(r.full_name(), "CorvidLabs/fledge-templates");
    }

    #[test]
    fn json_output_format() {
        let results = vec![SearchResult {
            owner: "test".to_string(),
            name: "tpl".to_string(),
            description: "A template".to_string(),
            stars: 5,
            url: "https://github.com/test/tpl".to_string(),
            topics: vec!["fledge-template".to_string()],
        }];
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&results).unwrap()).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["owner"], "test");
        assert_eq!(arr[0]["name"], "tpl");
        assert_eq!(arr[0]["stars"], 5);
    }

    #[test]
    fn to_json_has_canonical_search_entry_shape() {
        // This is the single source of truth every `*/search` command now emits.
        // If these keys change, update all three search rows in AGENTS.md.
        let r = SearchResult {
            owner: "CorvidLabs".to_string(),
            name: "fledge-plugin-github".to_string(),
            description: "PR workflows".to_string(),
            stars: 12,
            url: "https://github.com/CorvidLabs/fledge-plugin-github".to_string(),
            topics: vec![],
        };
        let entry = r.to_json("official");
        let obj = entry.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "description",
                "full_name",
                "name",
                "owner",
                "stars",
                "topics",
                "trust_tier",
                "url",
            ]
        );
        assert_eq!(entry["owner"], "CorvidLabs");
        assert_eq!(entry["full_name"], "CorvidLabs/fledge-plugin-github");
        assert_eq!(entry["trust_tier"], "official");
        // topics is present even when empty (documented as always-present).
        assert_eq!(entry["topics"], serde_json::json!([]));
    }

    #[test]
    fn urlencod_basic() {
        assert_eq!(urlencod("hello world"), "hello%20world");
        assert_eq!(urlencod("topic:fledge-template"), "topic%3Afledge-template");
        assert_eq!(urlencod("rust"), "rust");
    }

    #[test]
    fn parse_search_response_skips_items_without_owner() {
        let json = serde_json::json!({
            "total_count": 1,
            "items": [
                {
                    "name": "orphan",
                    "description": "No owner field",
                    "stargazers_count": 1,
                    "html_url": "https://github.com/x/orphan"
                }
            ]
        });

        let results = parse_search_response(&json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn parse_search_response_extracts_topics() {
        let json = serde_json::json!({
            "total_count": 1,
            "items": [
                {
                    "name": "fledge-rust-template",
                    "owner": { "login": "CorvidLabs" },
                    "description": "A Rust template",
                    "stargazers_count": 5,
                    "html_url": "https://github.com/CorvidLabs/fledge-rust-template",
                    "topics": ["fledge-template", "rust", "cli"]
                }
            ]
        });

        let results = parse_search_response(&json).unwrap();
        assert_eq!(results[0].topics, vec!["fledge-template", "rust", "cli"]);
    }
}
