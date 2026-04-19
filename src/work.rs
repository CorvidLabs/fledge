use anyhow::{Result, bail};
use console::style;
use std::process::Command;

#[derive(Debug)]
pub enum WorkAction {
    Start {
        name: String,
        base: Option<String>,
    },
    Pr {
        title: Option<String>,
        body: Option<String>,
        draft: bool,
        base: Option<String>,
    },
    Status,
}

pub fn run(action: WorkAction) -> Result<()> {
    ensure_git_repo()?;
    match action {
        WorkAction::Start { name, base } => start(&name, base.as_deref()),
        WorkAction::Pr {
            title,
            body,
            draft,
            base,
        } => pr(title.as_deref(), body.as_deref(), draft, base.as_deref()),
        WorkAction::Status => status(),
    }
}

fn ensure_git_repo() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !output.status.success() {
        bail!("Not a git repository. Run this command inside a git repo.");
    }
    Ok(())
}

fn git_output(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn default_branch() -> Result<String> {
    if let Ok(branch) = git_output(&["symbolic-ref", "refs/remotes/origin/HEAD", "--short"]) {
        if let Some(name) = branch.strip_prefix("origin/") {
            return Ok(name.to_string());
        }
        return Ok(branch);
    }

    for candidate in &["main", "master"] {
        if git_output(&["rev-parse", "--verify", candidate]).is_ok() {
            return Ok(candidate.to_string());
        }
    }

    Ok("main".to_string())
}

fn current_branch() -> Result<String> {
    git_output(&["branch", "--show-current"])
}

fn has_uncommitted_changes() -> Result<bool> {
    let output = git_output(&["status", "--porcelain"])?;
    Ok(!output.is_empty())
}

pub fn sanitize_branch_name(name: &str) -> String {
    let lowered = name.to_lowercase();
    let sanitized: String = lowered
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '/' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let mut result = String::new();
    let mut prev_hyphen = false;
    for c in sanitized.chars() {
        if c == '-' {
            if !prev_hyphen && !result.is_empty() {
                result.push(c);
                prev_hyphen = true;
            }
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    result.trim_end_matches('-').to_string()
}

fn start(name: &str, base: Option<&str>) -> Result<()> {
    if has_uncommitted_changes()? {
        bail!("Uncommitted changes detected. Commit or stash before starting work.");
    }

    let base_branch = match base {
        Some(b) => b.to_string(),
        None => default_branch()?,
    };

    let sanitized = sanitize_branch_name(name);
    let branch_name = if sanitized.contains('/') {
        sanitized
    } else {
        format!("feat/{sanitized}")
    };

    let existing = git_output(&["branch", "--list", &branch_name])?;
    if !existing.is_empty() {
        bail!("Branch '{branch_name}' already exists.");
    }

    git_output(&["checkout", "-b", &branch_name, &base_branch])?;

    println!(
        "{} Created branch {} from {}",
        style("✓").green().bold(),
        style(&branch_name).cyan(),
        style(&base_branch).dim()
    );
    println!(
        "{} Switched to {}",
        style("✓").green().bold(),
        style(&branch_name).cyan()
    );

    Ok(())
}

fn pr(title: Option<&str>, body: Option<&str>, draft: bool, base: Option<&str>) -> Result<()> {
    if Command::new("gh").arg("--version").output().is_err() {
        bail!(
            "GitHub CLI (gh) is not installed. Install it from https://cli.github.com/ and run `gh auth login`."
        );
    }

    let branch = current_branch()?;
    let default = default_branch()?;

    if branch == default || branch.is_empty() {
        bail!(
            "Cannot create a PR from the default branch '{}'. Switch to a feature branch first.",
            default
        );
    }

    let commits_ahead = commits_ahead_of(&branch, base.unwrap_or(&default))?;
    if commits_ahead == 0 {
        bail!(
            "No commits ahead of '{}'. Make some changes first.",
            base.unwrap_or(&default)
        );
    }

    let push_output = Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .output()?;
    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        bail!("Failed to push: {}", stderr.trim());
    }

    println!(
        "{} Pushed {} to origin",
        style("✓").green().bold(),
        style(&branch).cyan()
    );

    let pr_title = match title {
        Some(t) => t.to_string(),
        None => generate_title_from_branch(&branch),
    };

    let mut gh_args = vec![
        "pr".to_string(),
        "create".to_string(),
        "--title".to_string(),
        pr_title.clone(),
    ];

    if let Some(b) = body {
        gh_args.push("--body".to_string());
        gh_args.push(b.to_string());
    }

    if draft {
        gh_args.push("--draft".to_string());
    }

    if let Some(b) = base {
        gh_args.push("--base".to_string());
        gh_args.push(b.to_string());
    }

    let gh_output = Command::new("gh").args(&gh_args).output()?;

    if !gh_output.status.success() {
        let stderr = String::from_utf8_lossy(&gh_output.stderr);
        bail!("Failed to create PR: {}", stderr.trim());
    }

    let pr_url = String::from_utf8_lossy(&gh_output.stdout)
        .trim()
        .to_string();

    let draft_label = if draft { "draft " } else { "" };
    println!(
        "{} Created {}PR: \"{}\"",
        style("✓").green().bold(),
        draft_label,
        style(&pr_title).green()
    );
    println!("  {}", style(&pr_url).dim());

    Ok(())
}

fn status() -> Result<()> {
    let branch = current_branch()?;
    if branch.is_empty() {
        bail!("Detached HEAD — not on any branch.");
    }

    let default = default_branch()?;
    let ahead = commits_ahead_of(&branch, &default)?;

    println!(
        "  Branch: {} ({} {} ahead of {})",
        style(&branch).cyan(),
        ahead,
        if ahead == 1 { "commit" } else { "commits" },
        style(&default).dim()
    );

    if Command::new("gh").arg("--version").output().is_ok() {
        let gh_output = Command::new("gh")
            .args([
                "pr",
                "view",
                "--json",
                "number,state,url",
                "--jq",
                ".number,.state,.url",
            ])
            .output();

        match gh_output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.trim().lines().collect();
                if lines.len() >= 3 {
                    println!(
                        "  PR: #{} ({}) — {}",
                        style(lines[0]).green(),
                        lines[1].to_lowercase(),
                        style(lines[2]).dim()
                    );
                }
            }
            _ => {
                println!("  PR: {}", style("none").dim());
            }
        }
    }

    Ok(())
}

fn commits_ahead_of(branch: &str, base: &str) -> Result<usize> {
    let range = format!("{base}..{branch}");
    let output = git_output(&["rev-list", "--count", &range])?;
    Ok(output.parse().unwrap_or(0))
}

pub fn generate_title_from_branch(branch: &str) -> String {
    let name = branch
        .strip_prefix("feat/")
        .or_else(|| branch.strip_prefix("fix/"))
        .or_else(|| branch.strip_prefix("chore/"))
        .or_else(|| branch.strip_prefix("refactor/"))
        .unwrap_or(branch);

    let words: Vec<String> = name
        .split('-')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if words.is_empty() {
        return branch.to_string();
    }

    let mut title = String::new();
    for (i, word) in words.iter().enumerate() {
        if i == 0 {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                title.push_str(&first.to_uppercase().to_string());
                title.push_str(chars.as_str());
            }
        } else {
            title.push(' ');
            title.push_str(word);
        }
    }

    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_simple() {
        assert_eq!(sanitize_branch_name("add-search"), "add-search");
    }

    #[test]
    fn test_sanitize_spaces() {
        assert_eq!(sanitize_branch_name("add search"), "add-search");
    }

    #[test]
    fn test_sanitize_uppercase() {
        assert_eq!(sanitize_branch_name("Add-Search"), "add-search");
    }

    #[test]
    fn test_sanitize_special_chars() {
        assert_eq!(sanitize_branch_name("fix: bug #123"), "fix-bug-123");
    }

    #[test]
    fn test_sanitize_consecutive_hyphens() {
        assert_eq!(sanitize_branch_name("a--b---c"), "a-b-c");
    }

    #[test]
    fn test_sanitize_trailing_hyphens() {
        assert_eq!(sanitize_branch_name("test-"), "test");
    }

    #[test]
    fn test_sanitize_preserves_slashes() {
        assert_eq!(sanitize_branch_name("feat/my-thing"), "feat/my-thing");
    }

    #[test]
    fn test_generate_title_feat_prefix() {
        assert_eq!(
            generate_title_from_branch("feat/add-search-command"),
            "Add search command"
        );
    }

    #[test]
    fn test_generate_title_fix_prefix() {
        assert_eq!(
            generate_title_from_branch("fix/null-pointer"),
            "Null pointer"
        );
    }

    #[test]
    fn test_generate_title_no_prefix() {
        assert_eq!(
            generate_title_from_branch("my-cool-feature"),
            "My cool feature"
        );
    }

    #[test]
    fn test_generate_title_single_word() {
        assert_eq!(generate_title_from_branch("feat/search"), "Search");
    }

    #[test]
    fn test_generate_title_empty_after_prefix() {
        assert_eq!(generate_title_from_branch("feat/"), "feat/");
    }
}
