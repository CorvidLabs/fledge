use anyhow::Result;
use console::style;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub author: Option<String>,
    pub limit: usize,
    pub json: bool,
}

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
}

pub fn run(options: SearchOptions) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let sp = crate::spinner::Spinner::start("Searching GitHub for templates:");
    let results = search_github_ex(
        options.query.as_deref(),
        options.author.as_deref(),
        "fledge-template",
        token.as_deref(),
        options.limit,
    );
    sp.finish();
    let results = results?;

    if results.is_empty() {
        println!("No templates found.");
        return Ok(());
    }

    if options.json {
        let json = serde_json::to_string_pretty(&results)?;
        println!("{}", json);
    } else {
        println!("{}\n", style("Fledge templates on GitHub:").bold());
        for r in &results {
            let stars = format_stars(r.stars);
            let desc = if r.description.len() > 60 {
                format!("{}...", &r.description[..57])
            } else {
                r.description.clone()
            };
            let topic_str = if r.topics.is_empty() {
                String::new()
            } else {
                format!(" [{}]", r.topics.join(", "))
            };
            println!(
                "  {} {} {}{}",
                style(&r.full_name()).green(),
                style(format!("({})", stars)).dim(),
                style(&desc).dim(),
                style(&topic_str).cyan(),
            );
        }
        println!(
            "\n{}",
            style("Use: fledge init <name> -t <owner/repo>").dim()
        );
    }

    Ok(())
}

pub fn build_search_query_ex(keyword: Option<&str>, author: Option<&str>, topic: &str) -> String {
    let mut parts = Vec::new();
    if let Some(kw) = keyword {
        parts.push(format!("{kw} in:name,description,topics"));
    }
    parts.push(format!("topic:{topic}"));
    if let Some(a) = author {
        parts.push(format!("user:{a}"));
    }
    parts.join(" ")
}

pub fn search_github_ex(
    keyword: Option<&str>,
    author: Option<&str>,
    topic: &str,
    token: Option<&str>,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let query = build_search_query_ex(keyword, author, topic);
    let per_page = limit.min(100);
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page={}",
        urlencod(&query),
        per_page,
    );

    let mut request = ureq::get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "fledge-cli");

    if let Some(t) = token {
        request = request.header("Authorization", &format!("Bearer {}", t));
    }

    let mut response = request.call().map_err(|e| {
        if let ureq::Error::StatusCode(403) = e {
            anyhow::anyhow!(
                "GitHub API rate limit exceeded. Set a token with: fledge config set github.token <your-token>"
            )
        } else {
            anyhow::anyhow!("GitHub API request failed: {}", e)
        }
    })?;

    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("reading GitHub API response: {}", e))?;

    let body: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("parsing GitHub API response: {}", e))?;

    parse_search_response(&body)
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
            b' ' => "+".to_string(),
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
    fn urlencod_basic() {
        assert_eq!(urlencod("hello world"), "hello+world");
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

    #[ignore]
    #[test]
    fn live_search_returns_results() {
        let results = search_github_ex(None, None, "fledge-template", None, 5).unwrap();
        // May be empty if no repos have the topic yet — just ensure no error
        let _ = results;
    }
}
