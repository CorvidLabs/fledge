use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("fledge")
        .join("templates")
}

pub fn is_remote_ref(name: &str) -> bool {
    name.contains('/')
        && !name.contains(' ')
        && name.split('/').count() >= 2
        && name.split('/').all(|s| !s.is_empty())
}

pub fn parse_remote_ref(name: &str) -> (&str, &str, Option<&str>, Option<&str>) {
    // Split off @ref first: owner/repo@ref or owner/repo/subpath@ref
    let (name_part, git_ref) = match name.rsplit_once('@') {
        Some((before, after)) if !after.is_empty() => (before, Some(after)),
        _ => (name, None),
    };

    let parts: Vec<&str> = name_part.splitn(3, '/').collect();
    let owner = parts[0];
    let repo = parts[1];
    let subpath = parts.get(2).copied();
    (owner, repo, subpath, git_ref)
}

pub fn clear_cache(owner: &str, repo: &str) -> Result<()> {
    let repo_dir = cache_dir().join(owner).join(repo);
    if repo_dir.exists() {
        std::fs::remove_dir_all(&repo_dir)
            .with_context(|| format!("removing cached repo at {}", repo_dir.display()))?;
    }
    Ok(())
}

pub fn fetch_repo(
    owner: &str,
    repo: &str,
    token: Option<&str>,
    git_ref: Option<&str>,
) -> Result<PathBuf> {
    let cache = cache_dir();
    let ref_suffix = git_ref.unwrap_or("HEAD");
    let repo_dir = if git_ref.is_some() {
        cache.join(owner).join(format!("{}@{}", repo, ref_suffix))
    } else {
        cache.join(owner).join(repo)
    };

    if repo_dir.exists() {
        if git_ref.is_none() {
            update_repo(&repo_dir)?;
        }
    } else {
        clone_repo(owner, repo, token, &repo_dir, git_ref)?;
    }

    Ok(repo_dir)
}

fn clone_repo(
    owner: &str,
    repo: &str,
    token: Option<&str>,
    target: &Path,
    git_ref: Option<&str>,
) -> Result<()> {
    std::fs::create_dir_all(target.parent().unwrap_or(target))?;

    let url = repo_url(owner, repo, token);

    let mut args = vec!["clone", "--depth", "1"];
    if let Some(r) = git_ref {
        args.push("--branch");
        args.push(r);
    }
    args.push(&url);

    let status = std::process::Command::new("git")
        .args(&args)
        .arg(target)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("running git clone")?;

    if !status.success() {
        if let Some(r) = git_ref {
            bail!(
                "Failed to clone {}/{}@{}. Check the ref exists.",
                owner,
                repo,
                r
            );
        }
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
    git_ref: Option<&str>,
) -> Result<PathBuf> {
    let repo_dir = fetch_repo(owner, repo, token, git_ref)?;

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
        let (owner, repo, sub, git_ref) = parse_remote_ref("CorvidLabs/fledge-templates");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-templates");
        assert!(sub.is_none());
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_remote_ref_with_subpath() {
        let (owner, repo, sub, git_ref) = parse_remote_ref("CorvidLabs/fledge-templates/rust-cli");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-templates");
        assert_eq!(sub, Some("rust-cli"));
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_remote_ref_deep_subpath() {
        let (owner, repo, sub, git_ref) = parse_remote_ref("CorvidLabs/templates/lang/rust-cli");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "templates");
        assert_eq!(sub, Some("lang/rust-cli"));
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_remote_ref_with_version_tag() {
        let (owner, repo, sub, git_ref) = parse_remote_ref("CorvidLabs/my-template@v1.2.0");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "my-template");
        assert!(sub.is_none());
        assert_eq!(git_ref, Some("v1.2.0"));
    }

    #[test]
    fn parse_remote_ref_with_branch() {
        let (owner, repo, sub, git_ref) = parse_remote_ref("user/repo@main");
        assert_eq!(owner, "user");
        assert_eq!(repo, "repo");
        assert!(sub.is_none());
        assert_eq!(git_ref, Some("main"));
    }

    #[test]
    fn parse_remote_ref_subpath_with_ref() {
        let (owner, repo, sub, git_ref) = parse_remote_ref("CorvidLabs/templates/rust-cli@v2.0");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "templates");
        assert_eq!(sub, Some("rust-cli"));
        assert_eq!(git_ref, Some("v2.0"));
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

    #[test]
    fn clear_cache_removes_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fake_cache = tmp.path().join("owner").join("repo");
        std::fs::create_dir_all(&fake_cache).unwrap();
        std::fs::write(fake_cache.join("file.txt"), "data").unwrap();
        assert!(fake_cache.exists());
        std::fs::remove_dir_all(&fake_cache).unwrap();
        assert!(!fake_cache.exists());
    }

    #[test]
    fn clear_cache_nonexistent_is_ok() {
        let result = clear_cache("nonexistent-owner", "nonexistent-repo");
        assert!(result.is_ok());
    }
}
