use anyhow::{bail, Result};
use std::process::Command;
use std::time::Duration;

/// Default timeout for GitHub API requests. Without this, a wedged endpoint
/// or network drop hangs `lanes search`, `templates search`, `plugins search`,
/// `lanes import`, and the publish flows indefinitely.
const GITHUB_API_TIMEOUT: Duration = Duration::from_secs(30);

/// Base URL of the GitHub REST API.
const GITHUB_API_BASE: &str = "https://api.github.com";

/// The REST base URL every GitHub call is built on. A constant in release
/// builds; in test builds it may be redirected at a loopback mock server
/// (`test_support::GithubBaseGuard`) so the real request paths can be
/// exercised without touching the network.
pub(crate) fn api_base() -> String {
    #[cfg(test)]
    {
        if let Some(base) = crate::test_support::github_api_base_override() {
            return base;
        }
    }
    GITHUB_API_BASE.to_string()
}

fn github_api_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(GITHUB_API_TIMEOUT))
        .build()
        .into()
}

/// Build the full GitHub REST API URL for `path`, appending percent-encoded
/// query parameters. Split out from `github_api_get` so the URL assembly is
/// testable without issuing a request.
fn build_api_url(path: &str, query_params: &[(&str, &str)]) -> String {
    let mut url = format!("{}{}", api_base(), path);

    if !query_params.is_empty() {
        url.push('?');
        for (i, (k, v)) in query_params.iter().enumerate() {
            if i > 0 {
                url.push('&');
            }
            url.push_str(k);
            url.push('=');
            url.push_str(&crate::search::urlencod(v));
        }
    }

    url
}

/// Map a GitHub API HTTP status code to a user-facing error message with a
/// remediation hint. Returns `None` for statuses that have no special-cased
/// message (the caller then emits a generic "request failed" error carrying the
/// underlying transport error). Split out so the classification is testable
/// without a live endpoint.
fn github_status_error_message(status: u16, path: &str) -> Option<String> {
    match status {
        404 => {
            let repo_id = path
                .trim_start_matches('/')
                .split('/')
                .nth(2)
                .map(|r| {
                    let owner = path.trim_start_matches('/').split('/').nth(1).unwrap_or("?");
                    format!("{}/{}", owner, r)
                })
                .unwrap_or_else(|| path.to_string());
            Some(format!(
                "Not found (404) for {}.\nThe repo may not exist, or it may be private — in that case configure a token with 'repo' scope: fledge config set github.token <token>",
                repo_id
            ))
        }
        403 => Some(
            "GitHub API rate limit exceeded. Set a token with: fledge config set github.token <your-token>"
                .to_string(),
        ),
        _ => None,
    }
}

#[cfg(test)]
fn parse_repo_url(url: &str) -> Result<(String, String)> {
    // SSH: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        if let Some((owner, repo)) = rest.split_once('/') {
            return Ok((owner.to_string(), repo.to_string()));
        }
    }

    // HTTPS: https://github.com/owner/repo.git or https://token@github.com/owner/repo.git
    if url.contains("github.com") {
        let after_gh = url
            .split("github.com/")
            .nth(1)
            .or_else(|| url.split("github.com:").nth(1));

        if let Some(path) = after_gh {
            let path = path.strip_suffix(".git").unwrap_or(path);
            if let Some((owner, repo)) = path.split_once('/') {
                let repo = repo.split('/').next().unwrap_or(repo);
                return Ok((owner.to_string(), repo.to_string()));
            }
        }
    }

    let sanitized = if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            format!("{}://<redacted>{}", &url[..scheme_end], &url[at_pos..])
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };
    bail!(
        "Could not parse GitHub owner/repo from remote URL: {}",
        sanitized
    );
}

pub fn github_api_get(
    path: &str,
    token: Option<&str>,
    query_params: &[(&str, &str)],
) -> Result<serde_json::Value> {
    let url = build_api_url(path, query_params);

    let agent = github_api_agent();
    let mut request = agent
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "fledge-cli");

    if let Some(t) = token {
        request = request.header("Authorization", &format!("Bearer {}", t));
    }

    let mut response = request.call().map_err(|e| {
        if let ureq::Error::StatusCode(code) = &e {
            if let Some(msg) = github_status_error_message(*code, path) {
                return anyhow::anyhow!("{msg}");
            }
        }
        anyhow::anyhow!("GitHub API request failed: {}", e)
    })?;

    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("reading GitHub API response: {}", e))?;

    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parsing GitHub API response: {}", e))
}

pub fn ensure_git_repo() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !output.status.success() {
        bail!("Not a git repository.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_url() {
        let (owner, repo) = parse_repo_url("https://github.com/CorvidLabs/fledge.git").unwrap();
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge");
    }

    #[test]
    fn parse_https_url_no_git_suffix() {
        let (owner, repo) = parse_repo_url("https://github.com/CorvidLabs/fledge").unwrap();
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge");
    }

    #[test]
    fn parse_ssh_url() {
        let (owner, repo) = parse_repo_url("git@github.com:CorvidLabs/fledge.git").unwrap();
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge");
    }

    #[test]
    fn parse_ssh_url_no_git_suffix() {
        let (owner, repo) = parse_repo_url("git@github.com:CorvidLabs/fledge").unwrap();
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge");
    }

    #[test]
    fn parse_https_with_token() {
        let (owner, repo) =
            parse_repo_url("https://ghp_abc@github.com/CorvidLabs/fledge.git").unwrap();
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge");
    }

    #[test]
    fn parse_invalid_url_errors() {
        assert!(parse_repo_url("https://gitlab.com/user/repo").is_err());
    }

    #[test]
    fn ensure_git_repo_ok_inside_repo() {
        let repo = crate::test_support::TestRepo::init();
        repo.run_in(|| assert!(ensure_git_repo().is_ok()));
    }

    #[test]
    fn ensure_git_repo_errors_outside_repo() {
        let tmp = tempfile::tempdir().unwrap();
        crate::test_support::with_cwd(tmp.path(), || {
            assert!(ensure_git_repo().is_err());
        });
    }

    // ── URL building + error classification (no network) ───────────────────

    #[test]
    fn build_api_url_without_query() {
        assert_eq!(
            build_api_url("/repos/CorvidLabs/fledge", &[]),
            "https://api.github.com/repos/CorvidLabs/fledge"
        );
    }

    #[test]
    fn build_api_url_encodes_and_joins_query() {
        let url = build_api_url(
            "/search/repositories",
            &[("q", "topic:fledge-plugin lang:rust"), ("per_page", "5")],
        );
        // First param after '?', the rest joined with '&', values encoded.
        assert_eq!(
            url,
            format!(
                "https://api.github.com/search/repositories?q={}&per_page=5",
                crate::search::urlencod("topic:fledge-plugin lang:rust")
            )
        );
    }

    #[test]
    fn status_error_404_names_the_repo_and_hints_token() {
        let msg = github_status_error_message(404, "/repos/CorvidLabs/fledge").unwrap();
        assert!(
            msg.contains("CorvidLabs/fledge"),
            "should name the repo: {msg}"
        );
        assert!(
            msg.contains("'repo' scope"),
            "should hint the token scope: {msg}"
        );
    }

    #[test]
    fn status_error_404_falls_back_to_raw_path_without_repo_segment() {
        // A path with no owner/repo pair (e.g. /user) keeps the raw path.
        let msg = github_status_error_message(404, "/user").unwrap();
        assert!(msg.contains("/user"), "should fall back to the path: {msg}");
    }

    #[test]
    fn status_error_403_mentions_rate_limit() {
        let msg = github_status_error_message(403, "/anything").unwrap();
        assert!(msg.contains("rate limit"));
    }

    #[test]
    fn status_error_other_codes_are_uncategorized() {
        // Everything but 404/403 falls through to the generic "request failed".
        assert!(github_status_error_message(500, "/x").is_none());
        assert!(github_status_error_message(401, "/x").is_none());
        assert!(github_status_error_message(200, "/x").is_none());
    }

    // ── github_api_get against a loopback mock server ──────────────────────
    //
    // These drive the real request path (agent, headers, status mapping, JSON
    // decoding) with the API base redirected at a `MockHttpServer`, so nothing
    // leaves the machine. See `test_support::MockHttpServer`.

    use crate::test_support::{dead_port_url, GithubBaseGuard, MockHttpServer, MockResponse};

    #[test]
    fn api_get_parses_body_and_sends_headers() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/repos/CorvidLabs/fledge",
            MockResponse::json(
                200,
                r#"{"full_name":"CorvidLabs/fledge","stargazers_count":7}"#,
            ),
        );
        let _base = GithubBaseGuard::api(&server.url());

        let body = github_api_get("/repos/CorvidLabs/fledge", Some("ghp_tok"), &[]).unwrap();
        assert_eq!(body["full_name"], "CorvidLabs/fledge");
        assert_eq!(body["stargazers_count"], 7);

        let req = server.request("GET", "/repos/CorvidLabs/fledge").unwrap();
        assert_eq!(req.header("accept"), Some("application/vnd.github.v3+json"));
        assert_eq!(req.header("user-agent"), Some("fledge-cli"));
        assert_eq!(req.header("authorization"), Some("Bearer ghp_tok"));
    }

    #[test]
    fn api_get_without_token_sends_no_authorization() {
        let server = MockHttpServer::start();
        server.on("GET", "/rate_limit", MockResponse::json(200, "{}"));
        let _base = GithubBaseGuard::api(&server.url());

        github_api_get("/rate_limit", None, &[]).unwrap();
        assert!(server
            .request("GET", "/rate_limit")
            .unwrap()
            .header("authorization")
            .is_none());
    }

    #[test]
    fn api_get_sends_encoded_query_params() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/search/repositories",
            MockResponse::json(200, r#"{"items":[]}"#),
        );
        let _base = GithubBaseGuard::api(&server.url());

        github_api_get(
            "/search/repositories",
            None,
            &[("q", "topic:fledge-plugin cli"), ("per_page", "5")],
        )
        .unwrap();

        let req = server.request("GET", "/search/repositories").unwrap();
        assert_eq!(req.query, "q=topic%3Afledge-plugin%20cli&per_page=5");
    }

    #[test]
    fn api_get_404_returns_the_remediation_message() {
        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/ghost", MockResponse::empty(404));
        let _base = GithubBaseGuard::api(&server.url());

        let err = github_api_get("/repos/octo/ghost", None, &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("octo/ghost"), "unexpected error: {err}");
        assert!(err.contains("'repo' scope"), "unexpected error: {err}");
    }

    #[test]
    fn api_get_403_returns_the_rate_limit_message() {
        let server = MockHttpServer::start();
        server.on("GET", "/search/repositories", MockResponse::empty(403));
        let _base = GithubBaseGuard::api(&server.url());

        let err = github_api_get("/search/repositories", None, &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("rate limit"), "unexpected error: {err}");
    }

    #[test]
    fn api_get_other_status_falls_back_to_generic_error() {
        let server = MockHttpServer::start();
        // No route registered — every request gets the fallback response.
        server.fallback(MockResponse::empty(500));
        let _base = GithubBaseGuard::api(&server.url());

        let err = github_api_get("/x", None, &[]).unwrap_err().to_string();
        assert!(
            err.contains("GitHub API request failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn api_get_non_json_body_errors_on_parse() {
        let server = MockHttpServer::start();
        server.on("GET", "/x", MockResponse::text(200, "<html>nope</html>"));
        let _base = GithubBaseGuard::api(&server.url());

        let err = github_api_get("/x", None, &[]).unwrap_err().to_string();
        assert!(
            err.contains("parsing GitHub API response"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn api_get_unreachable_host_errors_without_hanging() {
        let _base = GithubBaseGuard::api(&dead_port_url());
        let err = github_api_get("/x", None, &[]).unwrap_err().to_string();
        assert!(
            err.contains("GitHub API request failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn search_results_flow_through_github_api_get() {
        // `plugins/lanes/templates search` all funnel through this helper into
        // `search::parse_search_response`; exercise the pair end to end.
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/search/repositories",
            MockResponse::json(
                200,
                r#"{"items":[{"name":"fledge-plugin-github","owner":{"login":"CorvidLabs"},
                    "description":"GitHub plugin","stargazers_count":12,
                    "html_url":"https://github.com/CorvidLabs/fledge-plugin-github",
                    "topics":["fledge-plugin"]}]}"#,
            ),
        );
        let _base = GithubBaseGuard::api(&server.url());

        let body = github_api_get(
            "/search/repositories",
            None,
            &[("q", "topic:fledge-plugin")],
        )
        .unwrap();
        let results = crate::search::parse_search_response(&body).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].full_name(), "CorvidLabs/fledge-plugin-github");
        assert_eq!(results[0].stars, 12);
        assert_eq!(results[0].topics, vec!["fledge-plugin".to_string()]);
    }
}
