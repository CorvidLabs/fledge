use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("~/.cache"))
        .join("fledge")
        .join("templates")
}

pub fn is_remote_ref(name: &str) -> bool {
    name.contains('/')
        && !name.contains(' ')
        && name.split('/').count() >= 2
        && name.split('/').all(|s| !s.is_empty())
}

pub fn parse_remote_ref(name: &str) -> (&str, &str, Option<&str>) {
    let parts: Vec<&str> = name.splitn(3, '/').collect();
    let owner = parts[0];
    let repo = parts[1];
    let subpath = parts.get(2).copied();
    (owner, repo, subpath)
}

pub fn fetch_repo(owner: &str, repo: &str, token: Option<&str>) -> Result<PathBuf> {
    let cache = cache_dir();
    let repo_dir = cache.join(owner).join(repo);

    if repo_dir.exists() {
        update_repo(&repo_dir)?;
    } else {
        clone_repo(owner, repo, token, &repo_dir)?;
    }

    Ok(repo_dir)
}

fn clone_repo(owner: &str, repo: &str, token: Option<&str>, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target.parent().unwrap_or(target))?;

    let url = repo_url(owner, repo, token);

    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &url])
        .arg(target)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("running git clone")?;

    if !status.success() {
        bail!(
            "Failed to clone {}/{}. Check the repo exists and you have access.",
            owner,
            repo
        );
    }

    Ok(())
}

fn update_repo(repo_dir: &Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["pull", "--ff-only", "--depth", "1"])
        .current_dir(repo_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running git pull")?;

    if !status.success() {
        let status = std::process::Command::new("git")
            .args(["fetch", "--depth", "1", "origin"])
            .current_dir(repo_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("running git fetch")?;

        if !status.success() {
            bail!("Failed to update cached repo at {}", repo_dir.display());
        }

        std::process::Command::new("git")
            .args(["reset", "--hard", "origin/HEAD"])
            .current_dir(repo_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("running git reset")?;
    }

    Ok(())
}

fn repo_url(owner: &str, repo: &str, token: Option<&str>) -> String {
    match token {
        Some(t) => format!("https://{}@github.com/{}/{}.git", t, owner, repo),
        None => format!("https://github.com/{}/{}.git", owner, repo),
    }
}

pub fn resolve_template_dir(
    owner: &str,
    repo: &str,
    subpath: Option<&str>,
    token: Option<&str>,
) -> Result<PathBuf> {
    let repo_dir = fetch_repo(owner, repo, token)?;

    match subpath {
        Some(sub) => {
            let template_dir = repo_dir.join(sub);
            if !template_dir.exists() {
                bail!("Subpath '{}' not found in {}/{}", sub, owner, repo);
            }
            Ok(template_dir)
        }
        None => Ok(repo_dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_remote_ref_owner_repo() {
        assert!(is_remote_ref("CorvidLabs/fledge-templates"));
    }

    #[test]
    fn is_remote_ref_with_subpath() {
        assert!(is_remote_ref("CorvidLabs/fledge-templates/rust-cli"));
    }

    #[test]
    fn is_remote_ref_rejects_simple_name() {
        assert!(!is_remote_ref("rust-cli"));
    }

    #[test]
    fn is_remote_ref_rejects_empty_segments() {
        assert!(!is_remote_ref("/repo"));
        assert!(!is_remote_ref("owner/"));
    }

    #[test]
    fn is_remote_ref_rejects_spaces() {
        assert!(!is_remote_ref("owner /repo"));
    }

    #[test]
    fn parse_remote_ref_owner_repo() {
        let (owner, repo, sub) = parse_remote_ref("CorvidLabs/fledge-templates");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-templates");
        assert!(sub.is_none());
    }

    #[test]
    fn parse_remote_ref_with_subpath() {
        let (owner, repo, sub) = parse_remote_ref("CorvidLabs/fledge-templates/rust-cli");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-templates");
        assert_eq!(sub, Some("rust-cli"));
    }

    #[test]
    fn parse_remote_ref_deep_subpath() {
        let (owner, repo, sub) = parse_remote_ref("CorvidLabs/templates/lang/rust-cli");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "templates");
        assert_eq!(sub, Some("lang/rust-cli"));
    }

    #[test]
    fn repo_url_without_token() {
        let url = repo_url("CorvidLabs", "fledge", None);
        assert_eq!(url, "https://github.com/CorvidLabs/fledge.git");
    }

    #[test]
    fn repo_url_with_token() {
        let url = repo_url("CorvidLabs", "fledge", Some("ghp_abc123"));
        assert_eq!(url, "https://ghp_abc123@github.com/CorvidLabs/fledge.git");
    }

    #[test]
    fn cache_dir_ends_with_expected_path() {
        let dir = cache_dir();
        assert!(dir.ends_with("fledge/templates"));
    }
}
