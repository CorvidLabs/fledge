use anyhow::{bail, Result};
use std::process::Command;
use std::time::Duration;

/// Default timeout for GitHub API requests. Without this, a wedged endpoint
/// or network drop hangs `lanes search`, `templates search`, `plugins search`,
/// `lanes import`, and the publish flows indefinitely.
const GITHUB_API_TIMEOUT: Duration = Duration::from_secs(30);

/// Base URL of the GitHub REST API.
const GITHUB_API_BASE: &str = "https://api.github.com";

/// Base URL of the `github.com/<owner>/<repo>.git` git endpoint — the clone
/// source for remote templates and the push target for every publish.
const GITHUB_REMOTE_BASE: &str = "https://github.com";

/// Test-only environment variable redirecting [`api_base`]. See
/// [`test_endpoint_override`] for the two gates that keep it out of a user's
/// way (debug builds only, loopback values only).
pub(crate) const API_BASE_ENV: &str = "FLEDGE_TEST_GITHUB_API_BASE";

/// Test-only environment variable redirecting [`remote_base`]. Same gates as
/// [`API_BASE_ENV`], plus it additionally accepts an existing local directory
/// (a tree of `<owner>/<repo>.git` bare repos standing in for github.com).
pub(crate) const REMOTE_BASE_ENV: &str = "FLEDGE_TEST_GITHUB_REMOTE_BASE";

/// The REST base URL every GitHub call is built on. `https://api.github.com`
/// unless a test has redirected it: in-process unit tests use the thread-local
/// `test_support::GithubBaseGuard`, and integration tests — which drive a
/// *spawned* `fledge` binary, where a thread-local cannot reach — use the
/// [`API_BASE_ENV`] environment variable.
pub(crate) fn api_base() -> String {
    #[cfg(test)]
    {
        if let Some(base) = crate::test_support::github_api_base_override() {
            return base;
        }
    }
    test_endpoint_override(API_BASE_ENV, false).unwrap_or_else(|| GITHUB_API_BASE.to_string())
}

/// The git remote base — `https://github.com` unless redirected the same two
/// ways [`api_base`] can be. Shared by `publish` (push target) and `remote`
/// (remote-template clone source) so both honour a single override.
pub(crate) fn remote_base() -> String {
    #[cfg(test)]
    {
        if let Some(base) = crate::test_support::github_remote_base_override() {
            return base;
        }
    }
    test_endpoint_override(REMOTE_BASE_ENV, true).unwrap_or_else(|| GITHUB_REMOTE_BASE.to_string())
}

/// The git URL for `owner/repo`: what `templates init <owner>/<repo>` clones
/// and what a publish pushes to.
pub(crate) fn remote_url(owner: &str, repo: &str) -> String {
    format!("{}/{}/{}.git", remote_base(), owner, repo)
}

/// Read a test-only endpoint override from the environment.
///
/// Integration tests spawn the real binary, so the `#[cfg(test)]` thread-local
/// overrides used by unit tests cannot reach them; without a runtime hook a
/// `TempEnv`-wrapped `templates search` would issue a real request to
/// api.github.com. Two independent gates keep that hook from being a
/// production attack surface:
///
/// 1. **Debug builds only.** Every binary users actually run (`cargo install`,
///    the release workflow, Homebrew, Nix) is compiled with `--release`, where
///    this function is the `None`-returning stub below and the constants above
///    are the only bases that exist.
/// 2. **Loopback-only values.** Even in a debug build the value must name
///    `127.0.0.1`, `[::1]` or `localhost` over plain `http` (userinfo such as
///    `http://127.0.0.1@evil.example/` is rejected — the host is what follows
///    the `@`), or, for the git remote base, an existing local directory.
///    Anything else is ignored with a warning on stderr. So even an
///    attacker-controlled environment cannot point an
///    `Authorization: Bearer <token>` request at a host off the machine.
#[cfg(debug_assertions)]
fn test_endpoint_override(var: &str, allow_local_dir: bool) -> Option<String> {
    let raw = std::env::var(var).ok()?;
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    if is_loopback_http_url(value) || (allow_local_dir && is_existing_local_dir(value)) {
        return Some(value.trim_end_matches('/').to_string());
    }
    eprintln!(
        "warning: ignoring {}: test endpoint overrides accept only a loopback http:// URL{}",
        var,
        if allow_local_dir {
            " or an existing local directory"
        } else {
            ""
        }
    );
    None
}

/// Release-build stub: the override does not exist in a shipped binary.
#[cfg(not(debug_assertions))]
fn test_endpoint_override(_var: &str, _allow_local_dir: bool) -> Option<String> {
    None
}

/// `true` for `http://` URLs whose *host* is loopback. Deliberately strict:
/// no `https`, no userinfo, no non-loopback host.
#[cfg(debug_assertions)]
fn is_loopback_http_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // `http://127.0.0.1@evil.example/` connects to *evil.example*: everything
    // before the `@` is userinfo, not the destination host.
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        // Bracketed IPv6 literal: `[::1]:8080`.
        Some(after) => match after.split_once(']') {
            Some((host, _port)) => host,
            None => return false,
        },
        None => authority.split(':').next().unwrap_or(""),
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

/// `true` for an absolute path that exists as a directory — a local stand-in
/// for github.com holding `<owner>/<repo>.git` bare repos.
#[cfg(debug_assertions)]
fn is_existing_local_dir(value: &str) -> bool {
    let path = std::path::Path::new(value);
    path.is_absolute() && path.is_dir()
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

    // ── the runtime (environment) endpoint override ────────────────────────
    //
    // The thread-local `GithubBaseGuard` cannot reach a spawned `fledge`
    // binary, so integration tests redirect the two GitHub endpoints with
    // environment variables instead. These tests pin down exactly how much
    // that hook is allowed to do — and that a release build ignores it.

    use crate::test_support::{
        dead_port_url, env_lock, EnvVarGuard, GithubBaseGuard, MockHttpServer, MockResponse,
    };

    #[test]
    fn endpoint_env_override_is_ignored_unless_set() {
        let _lock = env_lock();
        let _api = EnvVarGuard::set(API_BASE_ENV, None);
        let _remote = EnvVarGuard::set(REMOTE_BASE_ENV, None);

        assert_eq!(api_base(), GITHUB_API_BASE);
        assert_eq!(remote_base(), GITHUB_REMOTE_BASE);
        assert_eq!(
            remote_url("CorvidLabs", "fledge"),
            "https://github.com/CorvidLabs/fledge.git"
        );
    }

    #[test]
    fn endpoint_env_override_redirects_only_in_debug_builds() {
        let _lock = env_lock();
        let _api = EnvVarGuard::set(API_BASE_ENV, Some("http://127.0.0.1:9"));

        if cfg!(debug_assertions) {
            assert_eq!(api_base(), "http://127.0.0.1:9");
        } else {
            // Shipped binaries are release builds: the hook is compiled out
            // and the production constant is the only base that exists.
            assert_eq!(api_base(), GITHUB_API_BASE);
        }
    }

    #[test]
    fn endpoint_env_override_rejects_non_loopback_values() {
        let _lock = env_lock();
        // An override that could send `Authorization: Bearer <token>` off the
        // machine is refused even in a debug build — including the userinfo
        // trick, where the real host is what follows the `@`.
        for hostile in [
            "https://evil.example",
            "http://evil.example",
            "http://127.0.0.1@evil.example",
            "http://localhost@evil.example/api",
            "http://127.0.0.1.evil.example",
            "//127.0.0.1",
        ] {
            let _api = EnvVarGuard::set(API_BASE_ENV, Some(hostile));
            assert_eq!(
                api_base(),
                GITHUB_API_BASE,
                "{hostile} must not redirect the API base"
            );
        }
    }

    #[test]
    fn remote_env_override_accepts_loopback_and_local_dirs() {
        let _lock = env_lock();
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_str().unwrap().to_string();

        for (value, expected) in [
            ("http://127.0.0.1:9", Some("http://127.0.0.1:9".to_string())),
            ("http://[::1]:9", Some("http://[::1]:9".to_string())),
            (dir_path.as_str(), Some(dir_path.clone())),
            // A relative path or a path that does not exist is not a mock.
            ("some/relative/dir", None),
            ("https://evil.example", None),
        ] {
            let _remote = EnvVarGuard::set(REMOTE_BASE_ENV, Some(value));
            let expected = expected.unwrap_or_else(|| GITHUB_REMOTE_BASE.to_string()) + "/o/r.git";
            if cfg!(debug_assertions) {
                assert_eq!(remote_url("o", "r"), expected, "value: {value}");
            } else {
                assert_eq!(
                    remote_url("o", "r"),
                    "https://github.com/o/r.git",
                    "release builds ignore {value}"
                );
            }
        }
    }

    // ── github_api_get against a loopback mock server ──────────────────────
    //
    // These drive the real request path (agent, headers, status mapping, JSON
    // decoding) with the API base redirected at a `MockHttpServer`, so nothing
    // leaves the machine. See `test_support::MockHttpServer`.

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
