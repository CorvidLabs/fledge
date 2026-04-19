use anyhow::{Result, bail};
use std::process::Command;

pub fn detect_repo() -> Result<(String, String)> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()?;

    if !output.status.success() {
        bail!("No git remote 'origin' found. Are you in a GitHub repository?");
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_repo_url(&url)
}

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

    bail!("Could not parse GitHub owner/repo from remote URL: {}", url);
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

    let mut request = ureq::get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "fledge-cli");

    if let Some(t) = token {
        request = request.header("Authorization", &format!("Bearer {}", t));
    }

    let mut response = request.call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => anyhow::anyhow!("Not found: {}", path),
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

pub fn format_relative_time(iso: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return iso.to_string();
    };
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    let minutes = diff.num_minutes();
    if minutes < 1 {
        return "just now".to_string();
    }
    if minutes < 60 {
        return format!("{}m ago", minutes);
    }
    let hours = diff.num_hours();
    if hours < 24 {
        return format!("{}h ago", hours);
    }
    let days = diff.num_days();
    if days < 30 {
        return format!("{}d ago", days);
    }
    if days < 365 {
        return format!("{}mo ago", days / 30);
    }
    format!("{}y ago", days / 365)
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
    fn relative_time_minutes() {
        let now = chrono::Utc::now();
        let five_min_ago = now - chrono::Duration::minutes(5);
        let iso = five_min_ago.to_rfc3339();
        assert_eq!(format_relative_time(&iso), "5m ago");
    }

    #[test]
    fn relative_time_hours() {
        let now = chrono::Utc::now();
        let three_h_ago = now - chrono::Duration::hours(3);
        let iso = three_h_ago.to_rfc3339();
        assert_eq!(format_relative_time(&iso), "3h ago");
    }

    #[test]
    fn relative_time_days() {
        let now = chrono::Utc::now();
        let ten_d_ago = now - chrono::Duration::days(10);
        let iso = ten_d_ago.to_rfc3339();
        assert_eq!(format_relative_time(&iso), "10d ago");
    }

    #[test]
    fn relative_time_invalid_fallback() {
        assert_eq!(format_relative_time("not-a-date"), "not-a-date");
    }
}
