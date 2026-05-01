use anyhow::{bail, Result};
use std::process::Command;
use std::time::Duration;

/// Default timeout for GitHub API requests. Without this, a wedged endpoint
/// or network drop hangs `lanes search`, `templates search`, `plugins search`,
/// `lanes import`, and the publish flows indefinitely.
const GITHUB_API_TIMEOUT: Duration = Duration::from_secs(30);

fn github_api_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(GITHUB_API_TIMEOUT))
        .build()
        .into()
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
    let mut url = format!("https://api.github.com{}", path);

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

    let agent = github_api_agent();
    let mut request = agent
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "fledge-cli");

    if let Some(t) = token {
        request = request.header("Authorization", &format!("Bearer {}", t));
    }

    let mut response = request.call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => {
            let repo_id = path.trim_start_matches('/').split('/').nth(2).map(|r| {
                let owner = path.trim_start_matches('/').split('/').nth(1).unwrap_or("?");
                format!("{}/{}", owner, r)
            }).unwrap_or_else(|| path.to_string());
            anyhow::anyhow!(
                "Not found (404) for {}.\nThe repo may not exist, or it may be private — in that case configure a token with 'repo' scope: fledge config set github.token <token>",
                repo_id
            )
        }
        ureq::Error::StatusCode(403) => anyhow::anyhow!(
            "GitHub API rate limit exceeded. Set a token with: fledge config set github.token <your-token>"
        ),
        _ => anyhow::anyhow!("GitHub API request failed: {}", e),
    })?;

    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("reading GitHub API response: {}", e))?;

    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parsing GitHub API response: {}", e))
}

pub fn ensure_claude_cli() -> Result<()> {
    if Command::new("claude")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_err()
    {
        bail!(
            "Claude CLI is not installed. Install it from https://docs.anthropic.com/en/docs/claude-code and run `claude` to authenticate."
        );
    }
    Ok(())
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
}
