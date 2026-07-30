use anyhow::{bail, Context, Result};
use console::style;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default timeout for GitHub publish API requests. Without this, a wedged
/// endpoint hangs the publish flows indefinitely.
const PUBLISH_TIMEOUT: Duration = Duration::from_secs(30);

fn publish_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(PUBLISH_TIMEOUT))
        .build()
        .into()
}

/// Base URL for the `https://github.com/<owner>/<repo>.git` push remote.
const GITHUB_REMOTE_BASE: &str = "https://github.com";

/// The git remote base every publish pushes to. A constant in release builds;
/// test builds may redirect it at a directory of local bare repos
/// (`test_support::GithubBaseGuard`) so the push path runs without a network.
fn remote_base() -> String {
    #[cfg(test)]
    {
        if let Some(base) = crate::test_support::github_remote_base_override() {
            return base;
        }
    }
    GITHUB_REMOTE_BASE.to_string()
}

/// The push URL for `owner/repo`.
fn remote_url(owner: &str, repo: &str) -> String {
    format!("{}/{}/{}.git", remote_base(), owner, repo)
}

pub fn get_authenticated_user(token: &str) -> Result<String> {
    let agent = publish_agent();
    let text = agent
        .get(&format!("{}/user", crate::github::api_base()))
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fledge-cli")
        .call()
        .context("GitHub API request failed")?
        .body_mut()
        .read_to_string()
        .context("reading GitHub user response")?;

    let response: serde_json::Value =
        serde_json::from_str(&text).context("parsing GitHub user response")?;

    response["login"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not determine GitHub username"))
}

pub fn check_repo_exists(owner: &str, repo: &str, token: &str) -> Result<bool> {
    let url = format!("{}/repos/{}/{}", crate::github::api_base(), owner, repo);
    let agent = publish_agent();
    let result = agent
        .get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fledge-cli")
        .call();

    match result {
        Ok(_) => Ok(true),
        Err(ureq::Error::StatusCode(404)) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("GitHub API error: {}", e)),
    }
}

pub fn create_github_repo(
    name: &str,
    description: &str,
    private: bool,
    org: Option<&str>,
    token: &str,
) -> Result<()> {
    let base = crate::github::api_base();
    let url = match org {
        Some(o) => format!("{}/orgs/{}/repos", base, o),
        None => format!("{}/user/repos", base),
    };

    let body = json!({
        "name": name,
        "description": description,
        "private": private,
        "auto_init": false,
    });

    let json_body = serde_json::to_string(&body).context("serializing request body")?;

    let agent = publish_agent();
    let result = agent
        .post(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fledge-cli")
        .header("Content-Type", "application/json")
        .send(json_body.as_bytes());

    match result {
        Ok(_) => Ok(()),
        Err(ureq::Error::StatusCode(422)) => {
            bail!("Repository '{}' already exists or name is invalid", name)
        }
        Err(ureq::Error::StatusCode(403)) => {
            bail!("Permission denied. Check your token has 'repo' scope.")
        }
        Err(e) => bail!("Failed to create repository: {}", e),
    }
}

pub fn set_repo_topic(owner: &str, repo: &str, topic: &str, token: &str) -> Result<()> {
    let url = format!(
        "{}/repos/{}/{}/topics",
        crate::github::api_base(),
        owner,
        repo
    );

    let agent = publish_agent();
    let text = agent
        .get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fledge-cli")
        .call()
        .context("fetching repo topics")?
        .body_mut()
        .read_to_string()
        .context("reading topics response")?;

    let existing: serde_json::Value =
        serde_json::from_str(&text).context("parsing topics response")?;

    let mut topics: Vec<String> = existing["names"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if !topics.iter().any(|t| t == topic) {
        topics.push(topic.to_string());
    }

    let body = json!({ "names": topics });

    let json_body = serde_json::to_string(&body).context("serializing topics")?;

    agent
        .put(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fledge-cli")
        .header("Content-Type", "application/json")
        .send(json_body.as_bytes())
        .context("setting repo topics")?;

    Ok(())
}

pub fn push_directory(
    path: &Path,
    owner: &str,
    repo: &str,
    token: &str,
    commit_message: &str,
    json: bool,
) -> Result<()> {
    let git_dir = path.join(".git");
    let needs_init = !git_dir.exists();

    if needs_init {
        run_git(path, &["init"])?;
        run_git(path, &["checkout", "-b", "main"])?;
    }

    let remote_url = remote_url(owner, repo);

    let has_remote = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if has_remote {
        run_git(path, &["remote", "set-url", "origin", &remote_url])?;
    } else {
        run_git(path, &["remote", "add", "origin", &remote_url])?;
    }

    run_git(path, &["add", "-A"])?;

    let has_changes = !std::process::Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if has_changes {
        run_git(path, &["commit", "-m", commit_message])?;
    }

    use base64::Engine;
    let credentials = format!("x-access-token:{}", token);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&credentials);
    let header_value = format!("Authorization: Basic {}", encoded);

    let existing: usize = std::env::var("GIT_CONFIG_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if !json {
        println!(
            "{} Force-pushing to {}/{}...",
            style("*").cyan().bold(),
            owner,
            repo
        );
    }
    let output = std::process::Command::new("git")
        .args(["push", "-u", "origin", "main", "--force"])
        .current_dir(path)
        .env("GIT_CONFIG_COUNT", (existing + 1).to_string())
        .env(format!("GIT_CONFIG_KEY_{existing}"), "http.extraheader")
        .env(format!("GIT_CONFIG_VALUE_{existing}"), &header_value)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("running git push")?;

    if !output.status.success() {
        // Redact the injected token before surfacing git's stderr — the push
        // passes the GitHub token via http.extraheader, which git can echo back
        // in error output (matches the redaction boundary used in remote.rs).
        let stderr = crate::utils::redact_secrets(&String::from_utf8_lossy(&output.stderr));
        let detail = stderr.trim();
        if detail.is_empty() {
            bail!(
                "Failed to push to {}/{}. Check your token has 'repo' scope.",
                owner,
                repo
            );
        } else {
            bail!(
                "Failed to push to {}/{}. Check your token has 'repo' scope.\ngit error: {}",
                owner,
                repo,
                detail
            );
        }
    }

    if needs_init {
        // Clean up git remote URL to not embed token
        let clean_url = remote_url.clone();
        let _ = run_git(path, &["remote", "set-url", "origin", &clean_url]);
    }

    Ok(())
}

pub fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| format!("running git {}", args.join(" ")))?;

    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }

    Ok(())
}

/// Shared head of every `fledge <x> publish` flow: load config, require a GitHub
/// token, and canonicalize the source directory. Returns `(token, path)`.
///
/// Single-sourced so the three publish commands (`templates`/`plugins`/`lanes`)
/// cannot drift on the token/path error messages (issue #443).
pub fn publish_preflight(path: &Path) -> Result<(String, PathBuf)> {
    let config = crate::config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;
    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;
    Ok((token, path))
}

/// Resolve the repo owner: the `--org` value if given, else the authenticated
/// GitHub user (only then is `GET /user` hit, preserving org-vs-user behavior).
pub fn resolve_owner(org: Option<&str>, token: &str) -> Result<String> {
    match org {
        Some(o) => Ok(o.to_string()),
        None => get_authenticated_user(token),
    }
}

/// Everything the shared [`run_publish`] orchestration needs, carrying the
/// per-artifact differences (topic, commit message, envelope fields, and the
/// human-facing noun/verb/command). Built by each publish command from its own
/// manifest/config.
pub struct PublishRequest<'a> {
    pub path: &'a Path,
    pub owner: &'a str,
    pub repo_name: &'a str,
    pub description: &'a str,
    pub private: bool,
    pub org: Option<&'a str>,
    pub token: &'a str,
    pub yes: bool,
    pub json: bool,
    /// GitHub topic to tag the repo with, e.g. `fledge-template`.
    pub topic: &'a str,
    /// Git commit subject, e.g. `Publish fledge plugin`.
    pub commit_message: &'a str,
    /// Singular artifact noun for progress text: `Pushing {noun} files:`.
    pub noun: &'a str,
    pub schema_version: u32,
    /// Verb for the final tip: `Published! {verb} with:`.
    pub success_verb: &'a str,
    /// The command shown under the final tip.
    pub success_command: &'a str,
    /// Artifact-specific top-level envelope fields (e.g. `{"template": {...},
    /// "use_hint": "..."}`) merged alongside the shared `cancelled`/`repo`/`topic`.
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

/// Assemble the `publish` `--json` envelope. The shared keys (`cancelled`,
/// `repo`, `topic`) plus the request's `extra_fields` are merged; serde_json
/// sorts object keys, so the byte output matches the previous inline `json!`.
fn build_publish_envelope(
    req: &PublishRequest<'_>,
    cancelled: bool,
    created: bool,
) -> serde_json::Value {
    let mut fields = serde_json::Map::new();
    fields.insert("cancelled".to_string(), cancelled.into());
    fields.insert(
        "repo".to_string(),
        json!({
            "owner": req.owner,
            "name": req.repo_name,
            "url": format!("https://github.com/{}/{}", req.owner, req.repo_name),
            "created": created,
            "private": req.private,
        }),
    );
    fields.insert("topic".to_string(), req.topic.into());
    for (key, value) in &req.extra_fields {
        fields.insert(key.clone(), value.clone());
    }
    crate::envelope::action(
        req.schema_version,
        "publish",
        serde_json::Value::Object(fields),
    )
}

/// Shared publish orchestration tail: check-or-create the repo (honoring the
/// existing-repo confirmation prompt), set the topic, push the directory, and
/// emit the envelope or success text. Replaces the ~120 lines each of the three
/// publish commands used to duplicate (issue #443).
pub fn run_publish(req: PublishRequest<'_>) -> Result<()> {
    // JSON mode implies non-interactive consent (the confirm prompt below is
    // therefore never reached under --json — preserved existing behavior).
    let yes = req.yes || crate::utils::is_non_interactive() || req.json;

    let sp = if req.json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Checking repository:"))
    };
    let repo_exists = check_repo_exists(req.owner, req.repo_name, req.token)?;
    if let Some(s) = sp {
        s.finish();
    }

    let mut created_repo = false;
    if repo_exists {
        if !yes {
            crate::utils::require_interactive("yes")?;
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(format!(
                        "Repository {}/{} already exists. Push update?",
                        req.owner, req.repo_name
                    ))
                    .default(false)
                    .interact()?;

            if !confirm {
                if req.json {
                    let result = build_publish_envelope(&req, true, false);
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{} Cancelled.", style("*").cyan().bold());
                }
                return Ok(());
            }
        }
    } else {
        let sp = if req.json {
            None
        } else {
            Some(crate::spinner::Spinner::start("Creating repository:"))
        };
        create_github_repo(
            req.repo_name,
            req.description,
            req.private,
            req.org,
            req.token,
        )?;
        if let Some(s) = sp {
            s.finish();
        }
        created_repo = true;
        if !req.json {
            println!(
                "  {} Created repository {}/{}",
                style("✅").green().bold(),
                req.owner,
                req.repo_name
            );
        }
    }

    let sp = if req.json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Setting repository topics:"))
    };
    set_repo_topic(req.owner, req.repo_name, req.topic, req.token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !req.json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style(req.topic).cyan()
        );
    }

    let sp = if req.json {
        None
    } else {
        Some(crate::spinner::Spinner::start(&format!(
            "Pushing {} files:",
            req.noun
        )))
    };
    push_directory(
        req.path,
        req.owner,
        req.repo_name,
        req.token,
        req.commit_message,
        req.json,
    )?;
    if let Some(s) = sp {
        s.finish();
    }

    if req.json {
        let result = build_publish_envelope(&req, false, created_repo);
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("  {} Pushed {} files", style("✅").green().bold(), req.noun);
        println!(
            "\n{} Published! {} with:\n\n  {}",
            style("✅").green().bold(),
            req.success_verb,
            style(req.success_command).cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        dead_port_url, env_lock, GitIdentityGuard, GithubBaseGuard, MockHttpServer, MockResponse,
    };
    use serde_json::json;

    // Every test below drives the real HTTP / git code paths against a
    // loopback `MockHttpServer` and local bare repositories: no request ever
    // leaves the machine, and no test reads the developer's git or fledge
    // config. See `test_support::MockHttpServer` for the harness.

    // ── GET /user ─────────────────────────────────────────────────────────

    #[test]
    fn get_authenticated_user_returns_login_and_sends_auth() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/user",
            MockResponse::json(200, r#"{"login":"octo"}"#),
        );
        let _base = GithubBaseGuard::api(&server.url());

        assert_eq!(get_authenticated_user("ghp_secret").unwrap(), "octo");

        let req = server
            .request("GET", "/user")
            .expect("GET /user was issued");
        assert_eq!(req.header("authorization"), Some("Bearer ghp_secret"));
        assert_eq!(req.header("accept"), Some("application/vnd.github+json"));
        assert_eq!(req.header("user-agent"), Some("fledge-cli"));
    }

    #[test]
    fn get_authenticated_user_without_login_errors() {
        let server = MockHttpServer::start();
        server.on("GET", "/user", MockResponse::json(200, r#"{"id":1}"#));
        let _base = GithubBaseGuard::api(&server.url());

        let err = get_authenticated_user("t").unwrap_err().to_string();
        assert!(
            err.contains("Could not determine GitHub username"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn get_authenticated_user_on_unreachable_host_errors() {
        // Connection refused rather than a live endpoint — the error is the
        // contextual "GitHub API request failed", not a panic or a hang.
        let _base = GithubBaseGuard::api(&dead_port_url());
        let err = get_authenticated_user("t").unwrap_err().to_string();
        assert!(
            err.contains("GitHub API request failed"),
            "unexpected error: {err}"
        );
    }

    // ── GET /repos/{owner}/{repo} ─────────────────────────────────────────

    #[test]
    fn check_repo_exists_true() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/repos/octo/widget",
            MockResponse::json(200, r#"{"name":"widget"}"#),
        );
        let _base = GithubBaseGuard::api(&server.url());

        assert!(check_repo_exists("octo", "widget", "tok").unwrap());
        let req = server.request("GET", "/repos/octo/widget").unwrap();
        assert_eq!(req.header("authorization"), Some("Bearer tok"));
    }

    #[test]
    fn check_repo_exists_404() {
        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/widget", MockResponse::empty(404));
        let _base = GithubBaseGuard::api(&server.url());

        // A 404 is "not there yet", not a failure — the publish flow then
        // creates the repo.
        assert!(!check_repo_exists("octo", "widget", "tok").unwrap());
    }

    #[test]
    fn check_repo_exists_other_error() {
        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/widget", MockResponse::empty(500));
        let _base = GithubBaseGuard::api(&server.url());

        let err = check_repo_exists("octo", "widget", "tok")
            .unwrap_err()
            .to_string();
        assert!(err.contains("GitHub API error"), "unexpected error: {err}");
    }

    // ── POST /user/repos and /orgs/{org}/repos ────────────────────────────

    #[test]
    fn create_repo_request_body() {
        let server = MockHttpServer::start();
        server.on("POST", "/user/repos", MockResponse::empty(201));
        let _base = GithubBaseGuard::api(&server.url());

        create_github_repo("my-template", "A cool template", false, None, "tok").unwrap();

        let req = server.request("POST", "/user/repos").expect("POST issued");
        let body = req.json();
        assert_eq!(body["name"], "my-template");
        assert_eq!(body["description"], "A cool template");
        assert_eq!(body["private"], false);
        assert_eq!(body["auto_init"], false);
        assert_eq!(req.header("content-type"), Some("application/json"));
        assert_eq!(req.header("authorization"), Some("Bearer tok"));
    }

    #[test]
    fn create_repo_org_request() {
        let server = MockHttpServer::start();
        server.on("POST", "/orgs/CorvidLabs/repos", MockResponse::empty(201));
        let _base = GithubBaseGuard::api(&server.url());

        create_github_repo("t", "d", true, Some("CorvidLabs"), "tok").unwrap();

        // Org repos go to /orgs/<org>/repos, never /user/repos.
        let req = server.request("POST", "/orgs/CorvidLabs/repos").unwrap();
        assert_eq!(req.json()["private"], true);
        assert!(server.request("POST", "/user/repos").is_none());
    }

    #[test]
    fn create_repo_422_reports_name_conflict() {
        let server = MockHttpServer::start();
        server.on("POST", "/user/repos", MockResponse::empty(422));
        let _base = GithubBaseGuard::api(&server.url());

        let err = create_github_repo("dupe", "d", false, None, "tok")
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"), "unexpected error: {err}");
        assert!(err.contains("dupe"), "should name the repo: {err}");
    }

    #[test]
    fn create_repo_403_reports_missing_scope() {
        let server = MockHttpServer::start();
        server.on("POST", "/user/repos", MockResponse::empty(403));
        let _base = GithubBaseGuard::api(&server.url());

        let err = create_github_repo("t", "d", false, None, "tok")
            .unwrap_err()
            .to_string();
        assert!(err.contains("'repo' scope"), "unexpected error: {err}");
    }

    // ── PUT /repos/{owner}/{repo}/topics ──────────────────────────────────

    #[test]
    fn set_repo_topic_additive() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/repos/octo/widget/topics",
            MockResponse::json(200, r#"{"names":["rust","cli"]}"#),
        );
        server.on("PUT", "/repos/octo/widget/topics", MockResponse::empty(200));
        let _base = GithubBaseGuard::api(&server.url());

        set_repo_topic("octo", "widget", "fledge-template", "tok").unwrap();

        // The existing topics survive; the new one is appended.
        let put = server.request("PUT", "/repos/octo/widget/topics").unwrap();
        assert_eq!(
            put.json()["names"],
            json!(["rust", "cli", "fledge-template"])
        );
    }

    #[test]
    fn set_repo_topic_does_not_duplicate() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/repos/octo/widget/topics",
            MockResponse::json(200, r#"{"names":["fledge-lane","rust"]}"#),
        );
        server.on("PUT", "/repos/octo/widget/topics", MockResponse::empty(200));
        let _base = GithubBaseGuard::api(&server.url());

        set_repo_topic("octo", "widget", "fledge-lane", "tok").unwrap();

        let put = server.request("PUT", "/repos/octo/widget/topics").unwrap();
        assert_eq!(put.json()["names"], json!(["fledge-lane", "rust"]));
    }

    #[test]
    fn set_repo_topic_errors_when_fetch_fails() {
        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/widget/topics", MockResponse::empty(500));
        let _base = GithubBaseGuard::api(&server.url());

        let err = set_repo_topic("octo", "widget", "fledge-lane", "tok")
            .unwrap_err()
            .to_string();
        assert!(err.contains("fetching repo topics"), "unexpected: {err}");
    }

    // ── push_directory (real git, local bare remote) ──────────────────────

    /// Create `<root>/<owner>/<repo>.git` as a bare repo, so `remote_base()`
    /// pointed at `root` yields a pushable stand-in for github.com.
    fn init_bare_remote(root: &Path, owner: &str, repo: &str) -> PathBuf {
        let dir = root.join(owner).join(format!("{repo}.git"));
        std::fs::create_dir_all(&dir).unwrap();
        let out = std::process::Command::new("git")
            .args(["init", "--bare"])
            .current_dir(&dir)
            .output()
            .expect("spawn git init --bare");
        assert!(out.status.success(), "git init --bare failed");
        dir
    }

    fn git_out(bare: &Path, args: &[&str]) -> String {
        // `current_dir` is pinned to the repo: other tests in this binary
        // temporarily `chdir` into (and then delete) their own tempdirs, and a
        // git child inheriting a deleted cwd fails before it reads --git-dir.
        let out = std::process::Command::new("git")
            .current_dir(bare)
            .arg("--git-dir")
            .arg(bare)
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn push_directory_initializes_commits_and_pushes() {
        let _env = env_lock();
        let work = tempfile::tempdir().unwrap();
        let remotes = tempfile::tempdir().unwrap();
        let _git = GitIdentityGuard::new(work.path());
        let bare = init_bare_remote(remotes.path(), "octo", "widget");
        let _base =
            GithubBaseGuard::api_and_remote(&dead_port_url(), remotes.path().to_str().unwrap());

        std::fs::write(work.path().join("README.md"), "hello").unwrap();
        push_directory(
            work.path(),
            "octo",
            "widget",
            "ghp_secret",
            "Publish fledge plugin",
            true,
        )
        .unwrap();

        // The caller-supplied commit message is what lands on the remote.
        assert_eq!(
            git_out(&bare, &["log", "-1", "--pretty=%s", "main"]),
            "Publish fledge plugin"
        );
        assert!(git_out(&bare, &["ls-tree", "--name-only", "main"]).contains("README.md"));

        // The token is never persisted in the repo's git config; the remote is
        // reset to the clean URL after the push.
        let config = std::fs::read_to_string(work.path().join(".git/config")).unwrap();
        assert!(
            !config.contains("ghp_secret"),
            "token leaked into .git/config"
        );
        assert!(config.contains(remotes.path().to_str().unwrap()));

        // Second publish of the same directory takes the "repo already
        // initialized, remote already set" branch and pushes an update.
        std::fs::write(work.path().join("README.md"), "hello again").unwrap();
        push_directory(
            work.path(),
            "octo",
            "widget",
            "ghp_secret",
            "Publish fledge plugin",
            true,
        )
        .unwrap();
        assert_eq!(
            git_out(&bare, &["rev-list", "--count", "main"]),
            "2",
            "second push should add a commit"
        );
    }

    #[test]
    fn push_directory_surfaces_git_error() {
        let _env = env_lock();
        let work = tempfile::tempdir().unwrap();
        let remotes = tempfile::tempdir().unwrap();
        let _git = GitIdentityGuard::new(work.path());
        // No bare repo at <remotes>/octo/ghost.git — the push must fail.
        let _base =
            GithubBaseGuard::api_and_remote(&dead_port_url(), remotes.path().to_str().unwrap());

        std::fs::write(work.path().join("a.txt"), "x").unwrap();
        let err = push_directory(work.path(), "octo", "ghost", "tok", "Publish", true)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Failed to push to octo/ghost"),
            "unexpected error: {err}"
        );
        assert!(
            err.contains("git error:"),
            "should carry git's stderr: {err}"
        );
    }

    // ── run_publish orchestration (mock API + local remote) ───────────────

    fn publish_fixture<'a>(
        path: &'a Path,
        extra: serde_json::Map<String, serde_json::Value>,
    ) -> PublishRequest<'a> {
        PublishRequest {
            path,
            owner: "octo",
            repo_name: "widget",
            description: "desc",
            private: false,
            org: None,
            token: "ghp_secret",
            yes: true,
            json: true,
            topic: "fledge-plugin",
            commit_message: "Publish fledge plugin",
            noun: "plugin",
            schema_version: 1,
            success_verb: "Install",
            success_command: "fledge plugins install octo/widget",
            extra_fields: extra,
        }
    }

    #[test]
    fn run_publish_creates_repo_sets_topic_and_pushes() {
        let _env = env_lock();
        let work = tempfile::tempdir().unwrap();
        let remotes = tempfile::tempdir().unwrap();
        let _git = GitIdentityGuard::new(work.path());
        let bare = init_bare_remote(remotes.path(), "octo", "widget");

        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/widget", MockResponse::empty(404));
        server.on("POST", "/user/repos", MockResponse::empty(201));
        server.on(
            "GET",
            "/repos/octo/widget/topics",
            MockResponse::json(200, r#"{"names":[]}"#),
        );
        server.on("PUT", "/repos/octo/widget/topics", MockResponse::empty(200));
        let _base =
            GithubBaseGuard::api_and_remote(&server.url(), remotes.path().to_str().unwrap());

        std::fs::write(work.path().join("plugin.toml"), "name = \"widget\"").unwrap();
        run_publish(publish_fixture(work.path(), serde_json::Map::new())).unwrap();

        // Full flow: missing repo → created, topic set, content pushed.
        assert!(server.request("POST", "/user/repos").is_some());
        assert_eq!(
            server
                .request("PUT", "/repos/octo/widget/topics")
                .unwrap()
                .json()["names"],
            json!(["fledge-plugin"])
        );
        assert!(git_out(&bare, &["ls-tree", "--name-only", "main"]).contains("plugin.toml"));
    }

    #[test]
    fn run_publish_skips_creation_when_repo_exists() {
        let _env = env_lock();
        let work = tempfile::tempdir().unwrap();
        let remotes = tempfile::tempdir().unwrap();
        let _git = GitIdentityGuard::new(work.path());
        init_bare_remote(remotes.path(), "octo", "widget");

        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/widget", MockResponse::empty(200));
        server.on(
            "GET",
            "/repos/octo/widget/topics",
            MockResponse::json(200, r#"{"names":["rust"]}"#),
        );
        server.on("PUT", "/repos/octo/widget/topics", MockResponse::empty(200));
        let _base =
            GithubBaseGuard::api_and_remote(&server.url(), remotes.path().to_str().unwrap());

        std::fs::write(work.path().join("lane.toml"), "x = 1").unwrap();
        run_publish(publish_fixture(work.path(), serde_json::Map::new())).unwrap();

        // Existing repo: no creation call, and --json implies consent so the
        // confirmation prompt is never reached.
        assert!(server.request("POST", "/user/repos").is_none());
        assert!(server.request("PUT", "/repos/octo/widget/topics").is_some());
    }

    #[test]
    fn run_publish_stops_when_repo_check_fails() {
        let work = tempfile::tempdir().unwrap();
        let server = MockHttpServer::start();
        server.on("GET", "/repos/octo/widget", MockResponse::empty(500));
        let _base = GithubBaseGuard::api(&server.url());

        // A failed existence check aborts before any repo is created or pushed.
        assert!(run_publish(publish_fixture(work.path(), serde_json::Map::new())).is_err());
        assert!(server.request("POST", "/user/repos").is_none());
    }

    // ── resolve_owner ─────────────────────────────────────────────────────

    #[test]
    fn resolve_owner_prefers_org_without_calling_the_api() {
        let server = MockHttpServer::start();
        server.on(
            "GET",
            "/user",
            MockResponse::json(200, r#"{"login":"octo"}"#),
        );
        let _base = GithubBaseGuard::api(&server.url());

        assert_eq!(
            resolve_owner(Some("CorvidLabs"), "tok").unwrap(),
            "CorvidLabs"
        );
        assert!(
            server.requests().is_empty(),
            "GET /user must not be hit when --org is given"
        );

        assert_eq!(resolve_owner(None, "tok").unwrap(), "octo");
        assert!(server.request("GET", "/user").is_some());
    }

    fn sample_request<'a>(
        topic: &'a str,
        extra: serde_json::Map<String, serde_json::Value>,
    ) -> PublishRequest<'a> {
        PublishRequest {
            path: std::path::Path::new("/tmp/x"),
            owner: "octo",
            repo_name: "widget",
            description: "desc",
            private: false,
            org: None,
            token: "t",
            yes: true,
            json: true,
            topic,
            commit_message: "Publish",
            noun: "widget",
            schema_version: 1,
            success_verb: "Use",
            success_command: "cmd",
            extra_fields: extra,
        }
    }

    // The three envelope tests below prove the shared `build_publish_envelope`
    // produces byte-for-byte the same JSON the three publish commands used to
    // emit via their inline `json!` blocks (issue #443 dedup), on both the
    // success (created) and cancel paths — without touching the network.

    #[test]
    fn template_envelope_matches_legacy_inline_json() {
        let mut extra = serde_json::Map::new();
        extra.insert("template".to_string(), json!({ "description": "desc" }));
        extra.insert(
            "use_hint".to_string(),
            serde_json::Value::from("fledge templates init <name> --template octo/widget"),
        );
        let req = sample_request("fledge-template", extra);

        let expected_success = json!({
            "schema_version": 1,
            "action": "publish",
            "cancelled": false,
            "repo": {
                "owner": "octo",
                "name": "widget",
                "url": "https://github.com/octo/widget",
                "created": true,
                "private": false,
            },
            "template": { "description": "desc" },
            "topic": "fledge-template",
            "use_hint": "fledge templates init <name> --template octo/widget",
        });
        let got = build_publish_envelope(&req, false, true);
        assert_eq!(got, expected_success);
        assert_eq!(
            serde_json::to_string_pretty(&got).unwrap(),
            serde_json::to_string_pretty(&expected_success).unwrap()
        );

        let expected_cancel = json!({
            "schema_version": 1,
            "action": "publish",
            "cancelled": true,
            "repo": {
                "owner": "octo",
                "name": "widget",
                "url": "https://github.com/octo/widget",
                "created": false,
                "private": false,
            },
            "template": { "description": "desc" },
            "topic": "fledge-template",
            "use_hint": "fledge templates init <name> --template octo/widget",
        });
        assert_eq!(build_publish_envelope(&req, true, false), expected_cancel);
    }

    #[test]
    fn plugin_envelope_matches_legacy_inline_json() {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "plugin".to_string(),
            json!({ "name": "widget", "version": "0.1.0", "description": "desc" }),
        );
        extra.insert(
            "install_hint".to_string(),
            serde_json::Value::from("fledge plugins install octo/widget"),
        );
        let req = sample_request("fledge-plugin", extra);

        let expected = json!({
            "schema_version": 1,
            "action": "publish",
            "cancelled": false,
            "repo": {
                "owner": "octo",
                "name": "widget",
                "url": "https://github.com/octo/widget",
                "created": true,
                "private": false,
            },
            "plugin": { "name": "widget", "version": "0.1.0", "description": "desc" },
            "topic": "fledge-plugin",
            "install_hint": "fledge plugins install octo/widget",
        });
        assert_eq!(build_publish_envelope(&req, false, true), expected);
    }

    #[test]
    fn lanes_envelope_matches_legacy_inline_json() {
        let mut extra = serde_json::Map::new();
        extra.insert("lanes_published".to_string(), json!(["ci", "pre-commit"]));
        extra.insert(
            "import_hint".to_string(),
            serde_json::Value::from("fledge lanes import octo/widget"),
        );
        let req = sample_request("fledge-lane", extra);

        let expected = json!({
            "schema_version": 1,
            "action": "publish",
            "cancelled": false,
            "repo": {
                "owner": "octo",
                "name": "widget",
                "url": "https://github.com/octo/widget",
                "created": true,
                "private": false,
            },
            "lanes_published": ["ci", "pre-commit"],
            "topic": "fledge-lane",
            "import_hint": "fledge lanes import octo/widget",
        });
        assert_eq!(build_publish_envelope(&req, false, true), expected);
    }
}
