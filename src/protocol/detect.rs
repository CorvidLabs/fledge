use std::path::Path;
use std::process::Command;

use super::{GitContext, ProjectContext};

pub(crate) fn detect_project_context() -> Option<ProjectContext> {
    let root = std::env::current_dir().ok()?;

    let language = crate::run::detect_project_type(&root).to_string();

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git = detect_git_context(&root);

    Some(ProjectContext {
        name,
        root: root.to_string_lossy().to_string(),
        language,
        git,
    })
}

pub(crate) fn sanitize_remote_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some(at_pos) = rest.find('@') {
            return format!("https://{}", &rest[at_pos + 1..]);
        }
    } else if let Some(rest) = url.strip_prefix("http://") {
        if let Some(at_pos) = rest.find('@') {
            return format!("http://{}", &rest[at_pos + 1..]);
        }
    }
    url.to_string()
}

pub(crate) fn detect_git_context(root: &Path) -> Option<GitContext> {
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let remote = Command::new("git")
        .args(["remote"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("origin")
                .to_string()
        })
        .unwrap_or_else(|| "origin".to_string());

    let remote_url = Command::new("git")
        .args(["remote", "get-url", &remote])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| sanitize_remote_url(String::from_utf8_lossy(&o.stdout).trim()))
        .unwrap_or_default();

    Some(GitContext {
        branch,
        dirty,
        remote,
        remote_url,
    })
}
