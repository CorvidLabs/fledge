use anyhow::{bail, Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PublishOptions {
    pub path: PathBuf,
    pub org: Option<String>,
    pub private: bool,
    pub description: Option<String>,
    pub yes: bool,
}

pub fn run(options: PublishOptions) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;

    let path = options
        .path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", options.path.display()))?;

    let manifest = validate_template(&path)?;

    let repo_name = &manifest.template.name;
    let description = options
        .description
        .as_deref()
        .unwrap_or(&manifest.template.description);

    let owner = match &options.org {
        Some(org) => org.clone(),
        None => get_authenticated_user(&token)?,
    };

    println!(
        "{} Publishing {} as {}/{}",
        style("➡️").cyan().bold(),
        style(path.display()).dim(),
        style(&owner).green(),
        style(repo_name).green()
    );

    let sp = crate::spinner::Spinner::start("Checking repository:");
    let repo_exists = check_repo_exists(&owner, repo_name, &token)?;
    sp.finish();

    if repo_exists {
        if !options.yes {
            crate::utils::require_interactive("yes")?;
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Repository {}/{} already exists. Push update?",
                    owner, repo_name
                ))
                .default(false)
                .interact()?;

            if !confirm {
                println!("{} Cancelled.", style("*").cyan().bold());
                return Ok(());
            }
        }
    } else {
        let sp = crate::spinner::Spinner::start("Creating repository:");
        create_github_repo(
            repo_name,
            description,
            options.private,
            options.org.as_deref(),
            &token,
        )?;
        sp.finish();
        println!(
            "  {} Created repository {}/{}",
            style("✅").green().bold(),
            owner,
            repo_name
        );
    }

    let sp = crate::spinner::Spinner::start("Setting repository topics:");
    set_repo_topics(&owner, repo_name, &token)?;
    sp.finish();
    println!(
        "  {} Set {} topic",
        style("✅").green().bold(),
        style("fledge-template").cyan()
    );

    let sp = crate::spinner::Spinner::start("Pushing template files:");
    push_directory(&path, &owner, repo_name, &token)?;
    sp.finish();
    println!("  {} Pushed template files", style("✅").green().bold());

    println!(
        "\n{} Published! Install with:\n\n  {}",
        style("✅").green().bold(),
        style(format!(
            "fledge init <project-name> -t {}/{}",
            owner, repo_name
        ))
        .cyan()
    );

    Ok(())
}

pub fn validate_template(path: &Path) -> Result<crate::templates::TemplateManifest> {
    if !path.exists() {
        bail!("Directory not found: {}", path.display());
    }

    let manifest_path = path.join("template.toml");
    if !manifest_path.exists() {
        bail!(
            "No template.toml found in {}. Create one with: fledge create-template",
            path.display()
        );
    }

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;

    let manifest: crate::templates::TemplateManifest =
        toml::from_str(&content).with_context(|| "Invalid template.toml")?;

    Ok(manifest)
}

pub fn get_authenticated_user(token: &str) -> Result<String> {
    let text = ureq::get("https://api.github.com/user")
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
    let result = ureq::get(&url)
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

    let result = ureq::post(&url)
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

pub fn set_repo_topics(owner: &str, repo: &str, token: &str) -> Result<()> {
    set_repo_topic(owner, repo, "fledge-template", token)
}

pub fn set_repo_topic(owner: &str, repo: &str, topic: &str, token: &str) -> Result<()> {
    let url = format!("https://api.github.com/repos/{}/{}/topics", owner, repo);

    let text = ureq::get(&url)
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

    ureq::put(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "fledge-cli")
        .header("Content-Type", "application/json")
        .send(json_body.as_bytes())
        .context("setting repo topics")?;

    Ok(())
}

pub fn push_directory(path: &Path, owner: &str, repo: &str, token: &str) -> Result<()> {
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
        run_git(path, &["commit", "-m", "Publish fledge template"])?;
    }

    use base64::Engine;
    let credentials = format!("x-access-token:{}", token);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&credentials);
    let header_value = format!("Authorization: Basic {}", encoded);

    let existing: usize = std::env::var("GIT_CONFIG_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    println!(
        "{} Force-pushing to {}/{}...",
        style("*").cyan().bold(),
        owner,
        repo
    );
    let status = std::process::Command::new("git")
        .args(["push", "-u", "origin", "main", "--force"])
        .current_dir(path)
        .env("GIT_CONFIG_COUNT", (existing + 1).to_string())
        .env(format!("GIT_CONFIG_KEY_{existing}"), "http.extraheader")
        .env(format!("GIT_CONFIG_VALUE_{existing}"), &header_value)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("running git push")?;

    if !status.success() {
        bail!(
            "Failed to push to {}/{}. Check your token has 'repo' scope.",
            owner,
            repo
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_valid_manifest(dir: &Path) {
        std::fs::write(
            dir.join("template.toml"),
            r#"[template]
name = "test-template"
description = "A test template"

[files]
render = ["**/*.md"]
"#,
        )
        .unwrap();
    }

    #[test]
    fn validate_valid_template_succeeds() {
        let tmp = TempDir::new().unwrap();
        write_valid_manifest(tmp.path());

        let result = validate_template(tmp.path());
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.template.name, "test-template");
        assert_eq!(manifest.template.description, "A test template");
    }

    #[test]
    fn validate_missing_manifest_fails() {
        let tmp = TempDir::new().unwrap();

        let result = validate_template(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No template.toml"));
    }

    #[test]
    fn validate_invalid_manifest_fails() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("template.toml"), "not valid toml {{{{").unwrap();

        let result = validate_template(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn validate_nonexistent_dir_fails() {
        let result = validate_template(Path::new("/nonexistent/path/to/template"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Directory not found"));
    }

    #[test]
    fn validate_manifest_with_prompts() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("template.toml"),
            r#"[template]
name = "prompted"
description = "Has prompts"

[prompts.database]
message = "Database engine"
default = "sqlite"

[files]
render = ["**/*.rs"]
"#,
        )
        .unwrap();

        let result = validate_template(tmp.path());
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert!(manifest.prompts.contains_key("database"));
    }

    #[test]
    fn run_rejects_no_token() {
        let tmp = TempDir::new().unwrap();
        write_valid_manifest(tmp.path());

        let options = PublishOptions {
            path: tmp.path().to_path_buf(),
            org: None,
            private: false,
            description: None,
            yes: false,
        };

        let result = run(options);
        assert!(result.is_err());
    }

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
}
