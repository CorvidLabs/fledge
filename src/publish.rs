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

pub fn get_authenticated_user(token: &str) -> Result<String> {
    let agent = publish_agent();
    let text = agent
        .get("https://api.github.com/user")
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
    let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
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
    let url = match org {
        Some(o) => format!("https://api.github.com/orgs/{}/repos", o),
        None => "https://api.github.com/user/repos".to_string(),
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
    let url = format!("https://api.github.com/repos/{}/{}/topics", owner, repo);

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

    let remote_url = format!("https://github.com/{}/{}.git", owner, repo);

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
        let clean_url = format!("https://github.com/{}/{}.git", owner, repo);
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

    #[test]
    fn topics_include_fledge_template() {
        let mut topics: Vec<String> = vec!["rust".to_string(), "cli".to_string()];
        if !topics.iter().any(|t| t == "fledge-template") {
            topics.push("fledge-template".to_string());
        }
        assert!(topics.contains(&"fledge-template".to_string()));

        // Already present — should not duplicate
        let mut topics2: Vec<String> = vec!["fledge-template".to_string(), "rust".to_string()];
        if !topics2.iter().any(|t| t == "fledge-template") {
            topics2.push("fledge-template".to_string());
        }
        assert_eq!(
            topics2.iter().filter(|t| *t == "fledge-template").count(),
            1
        );
    }

    #[test]
    fn create_repo_request_body() {
        let body = serde_json::json!({
            "name": "my-template",
            "description": "A cool template",
            "private": false,
            "auto_init": false,
        });

        assert_eq!(body["name"], "my-template");
        assert_eq!(body["description"], "A cool template");
        assert_eq!(body["private"], false);
        assert_eq!(body["auto_init"], false);
    }

    #[test]
    fn create_repo_org_request_url() {
        let url = match Some("CorvidLabs") {
            Some(o) => format!("https://api.github.com/orgs/{}/repos", o),
            None => "https://api.github.com/user/repos".to_string(),
        };
        assert_eq!(url, "https://api.github.com/orgs/CorvidLabs/repos");

        let personal_url = match None::<&str> {
            Some(o) => format!("https://api.github.com/orgs/{}/repos", o),
            None => "https://api.github.com/user/repos".to_string(),
        };
        assert_eq!(personal_url, "https://api.github.com/user/repos");
    }

    #[test]
    #[ignore] // Requires GitHub token and network
    fn publish_live() {
        // Integration test: publish a real template
        // Run with: cargo test publish_live -- --ignored
    }

    use super::{build_publish_envelope, PublishRequest};
    use serde_json::json;

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
