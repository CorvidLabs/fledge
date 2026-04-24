use anyhow::{bail, Context, Result};
use console::style;
use serde::Deserialize;
use std::process::Command;

use crate::config::Config;

const VALID_BRANCH_TYPES: &[&str] = &[
    "feat", "feature", "fix", "bug", "chore", "task", "docs", "hotfix", "refactor",
];

#[derive(Debug, Deserialize)]
pub struct WorkConfig {
    #[serde(default = "default_branch_format")]
    pub branch_format: String,
    #[serde(default = "default_type")]
    pub default_type: String,
    pub branch_types: Option<Vec<String>>,
}

impl Default for WorkConfig {
    fn default() -> Self {
        Self {
            branch_format: default_branch_format(),
            default_type: default_type(),
            branch_types: None,
        }
    }
}

fn default_branch_format() -> String {
    "{author}/{type}/{name}".to_string()
}

fn default_type() -> String {
    "feat".to_string()
}

#[derive(Debug, Deserialize)]
struct FledgeWorkFile {
    #[serde(default)]
    work: WorkConfig,
}

fn load_work_config() -> WorkConfig {
    let cwd = std::env::current_dir().unwrap_or_default();
    let path = cwd.join("fledge.toml");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            match toml::from_str::<FledgeWorkFile>(&content) {
                Ok(file) => return file.work,
                Err(e) => eprintln!("Warning: failed to parse fledge.toml: {e}"),
            }
        }
    }
    WorkConfig::default()
}

#[derive(Debug)]
pub enum WorkAction {
    Start {
        name: String,
        branch_type: Option<String>,
        issue: Option<u64>,
        prefix: Option<String>,
        base: Option<String>,
        json: bool,
    },
    Pr {
        title: Option<String>,
        body: Option<String>,
        draft: bool,
        base: Option<String>,
        json: bool,
        yes: bool,
        ai: bool,
        provider: Option<String>,
        model: Option<String>,
    },
    Status {
        json: bool,
    },
}

pub fn run(action: WorkAction) -> Result<()> {
    crate::github::ensure_git_repo()?;
    match action {
        WorkAction::Start {
            name,
            branch_type,
            issue,
            prefix,
            base,
            json,
        } => start(
            &name,
            branch_type.as_deref(),
            issue,
            prefix.as_deref(),
            base.as_deref(),
            json,
        ),
        WorkAction::Pr {
            title,
            body,
            draft,
            base,
            json,
            yes,
            ai,
            provider,
            model,
        } => pr(
            title.as_deref(),
            body.as_deref(),
            draft,
            base.as_deref(),
            json,
            yes,
            ai,
            provider.as_deref(),
            model.as_deref(),
        ),
        WorkAction::Status { json } => status(json),
    }
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

fn start(
    name: &str,
    branch_type: Option<&str>,
    issue: Option<u64>,
    prefix: Option<&str>,
    base: Option<&str>,
    json: bool,
) -> Result<()> {
    if has_uncommitted_changes()? {
        bail!("Uncommitted changes detected. Commit or stash before starting work.");
    }

    let config = load_work_config();

    let btype = branch_type.unwrap_or(&config.default_type);
    let valid_types: Vec<&str> = if let Some(ref custom) = config.branch_types {
        custom.iter().map(|s| s.as_str()).collect()
    } else {
        VALID_BRANCH_TYPES.to_vec()
    };
    if prefix.is_none() && !valid_types.contains(&btype) {
        bail!(
            "Unknown branch type '{}'. Valid types: {}",
            btype,
            valid_types.join(", ")
        );
    }

    let base_branch = match base {
        Some(b) => b.to_string(),
        None => default_branch()?,
    };

    let sanitized = sanitize_branch_name(name);

    let author = Config::load()
        .ok()
        .and_then(|c| c.author_or_git())
        .map(|a| sanitize_branch_name(&a))
        .unwrap_or_default();

    if author.is_empty() && config.branch_format.contains("{author}") {
        bail!(
            "No author configured for branch format '{}'. Set one with `fledge config set defaults.author <name>` or configure git user.name.",
            config.branch_format
        );
    }

    let branch_name = if let Some(pfx) = prefix {
        format!("{}/{sanitized}", pfx.trim_end_matches('/'))
    } else {
        let name_part = match issue {
            Some(num) => format!("{num}-{sanitized}"),
            None => sanitized,
        };
        config
            .branch_format
            .replace("{author}", &author)
            .replace("{type}", btype)
            .replace("{issue}", &issue.map(|n| n.to_string()).unwrap_or_default())
            .replace("{name}", &name_part)
    };

    let existing = git_output(&["branch", "--list", &branch_name])?;
    if !existing.is_empty() {
        bail!("Branch '{branch_name}' already exists.");
    }

    git_output(&["checkout", "-b", &branch_name, &base_branch])?;

    crate::plugin::run_lifecycle_hook("post_work_start").ok();

    if json {
        let payload = serde_json::json!({
            "branch": branch_name,
            "base": base_branch,
            "type": btype,
            "prefix": prefix,
            "issue": issue,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "{} Created branch {} from {}",
            style("✅").green().bold(),
            style(&branch_name).cyan(),
            style(&base_branch).dim()
        );
        println!(
            "{} Switched to {}",
            style("✅").green().bold(),
            style(&branch_name).cyan()
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn pr(
    title: Option<&str>,
    body: Option<&str>,
    draft: bool,
    base: Option<&str>,
    json: bool,
    yes: bool,
    ai: bool,
    provider_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<()> {
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

    let base_branch = base.unwrap_or(&default).to_string();
    let commits_ahead = commits_ahead_of(&branch, &base_branch)?;
    if commits_ahead == 0 {
        bail!(
            "No commits ahead of '{}'. Make some changes first.",
            base_branch
        );
    }

    let pr_title = match title {
        Some(t) => t.to_string(),
        None => generate_title_from_branch(&branch),
    };
    let pr_body = match body {
        Some(b) => b.to_string(),
        None if ai => generate_body_with_ai(
            &branch,
            &base_branch,
            provider_override,
            model_override,
            json,
        )?,
        None => generate_body_from_commits(&branch, &base_branch)?,
    };

    if !json {
        print_pr_preview(&pr_title, &pr_body, &branch, &base_branch, draft);
        if !yes {
            if !crate::utils::is_interactive() {
                bail!(
                    "Refusing to create a PR without confirmation in a non-interactive shell. Re-run with --yes to skip the prompt."
                );
            }
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Create this pull request?")
                    .default(true)
                    .interact()?;
            if !confirm {
                println!("{} Aborted.", style("✋").yellow());
                return Ok(());
            }
        }
    }

    crate::plugin::run_lifecycle_hook("pre_pr")?;

    let push_sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start(&format!(
            "Pushing {} to origin:",
            &branch
        )))
    };
    let push_output = Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .output()?;
    if let Some(sp) = push_sp {
        sp.finish();
    }
    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        bail!("Failed to push: {}", stderr.trim());
    }

    if !json {
        println!(
            "{} Pushed {} to origin",
            style("✅").green().bold(),
            style(&branch).cyan()
        );
    }

    let mut gh_args = vec![
        "pr".to_string(),
        "create".to_string(),
        "--title".to_string(),
        pr_title.clone(),
        "--body".to_string(),
        pr_body.clone(),
    ];

    if draft {
        gh_args.push("--draft".to_string());
    }

    if let Some(b) = base {
        gh_args.push("--base".to_string());
        gh_args.push(b.to_string());
    }

    let create_sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start("Creating pull request:"))
    };
    let gh_output = Command::new("gh").args(&gh_args).output()?;
    if let Some(sp) = create_sp {
        sp.finish();
    }

    if !gh_output.status.success() {
        let stderr = String::from_utf8_lossy(&gh_output.stderr);
        bail!("Failed to create PR: {}", stderr.trim());
    }

    let pr_url = String::from_utf8_lossy(&gh_output.stdout)
        .trim()
        .to_string();
    let pr_number = extract_pr_number(&pr_url);

    if json {
        let payload = serde_json::json!({
            "url": pr_url,
            "number": pr_number,
            "title": pr_title,
            "head": branch,
            "base": base_branch,
            "draft": draft,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        let draft_label = if draft { "draft " } else { "" };
        println!(
            "{} Created {}PR: \"{}\"",
            style("✅").green().bold(),
            draft_label,
            style(&pr_title).green()
        );
        println!("  {}", style(&pr_url).dim());
    }

    Ok(())
}

fn status(json: bool) -> Result<()> {
    let branch = current_branch()?;
    if branch.is_empty() {
        bail!("Detached HEAD — not on any branch.");
    }

    let default = default_branch()?;
    let ahead = commits_ahead_of(&branch, &default)?;
    // `behind` needs the remote-tracking base; returns None if the base hasn't
    // been fetched. That's distinct from "actually 0 behind" and agents need
    // to tell them apart.
    let behind: Option<usize> = commits_behind_of(&branch, &default).ok();

    let pr_info = if Command::new("gh").arg("--version").output().is_ok() {
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
                    let number: Option<u64> = lines[0].parse().ok();
                    Some((number, lines[1].to_lowercase(), lines[2].to_string()))
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };

    if json {
        let pr_payload = match &pr_info {
            Some((number, state, url)) => serde_json::json!({
                "number": number,
                "state": state,
                "url": url,
            }),
            None => serde_json::Value::Null,
        };
        let payload = serde_json::json!({
            "branch": branch,
            "default": default,
            "ahead": ahead,
            "behind": behind,  // null when rev-list fails (e.g. base not fetched)
            "pr": pr_payload,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        let behind_display = match behind {
            Some(n) if n > 0 => format!(", {} behind", n),
            _ => String::new(),
        };
        println!(
            "  Branch: {} ({} {} ahead of {}{})",
            style(&branch).cyan(),
            ahead,
            if ahead == 1 { "commit" } else { "commits" },
            style(&default).dim(),
            behind_display,
        );
        match &pr_info {
            Some((number, state, url)) => {
                let number_display = number
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  PR: #{} ({}) — {}",
                    style(number_display).green(),
                    state,
                    style(url).dim()
                );
            }
            None => {
                println!("  PR: {}", style("none").dim());
            }
        }
    }

    Ok(())
}

fn commits_behind_of(branch: &str, base: &str) -> Result<usize> {
    let range = format!("{branch}..{base}");
    let output = git_output(&["rev-list", "--count", &range])?;
    Ok(output.parse().unwrap_or(0))
}

fn extract_pr_number(url: &str) -> Option<u64> {
    // GitHub PR URLs always contain `/pull/<n>`. Anchor on that rather than
    // the last path segment so trailing `/`, `?query`, or `#fragment` don't
    // silently turn into `None`.
    let after_pull = url.rsplit_once("/pull/")?.1;
    let end = after_pull.find(['/', '?', '#']).unwrap_or(after_pull.len());
    after_pull[..end].parse().ok()
}

fn commits_ahead_of(branch: &str, base: &str) -> Result<usize> {
    let range = format!("{base}..{branch}");
    let output = git_output(&["rev-list", "--count", &range])?;
    Ok(output.parse().unwrap_or(0))
}

/// Build a Markdown PR body from commit subjects between `base..branch`.
///
/// Format: `## Summary` heading + one bullet per commit (newest first), with
/// any `type:` conventional-commit prefix stripped and the leading char
/// upper-cased so bullets read like sentences. Falls back to a placeholder if
/// the rev-list call returns nothing.
pub fn generate_body_from_commits(branch: &str, base: &str) -> Result<String> {
    let range = format!("{base}..{branch}");
    let raw = git_output(&["log", "--pretty=format:%s", &range]).unwrap_or_default();

    let bullets: Vec<String> = raw
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(format_commit_subject_as_bullet)
        .collect();

    let body = if bullets.is_empty() {
        "## Summary\n\n- (describe the change)\n".to_string()
    } else {
        let mut out = String::from("## Summary\n\n");
        for bullet in &bullets {
            out.push_str("- ");
            out.push_str(bullet);
            out.push('\n');
        }
        out
    };

    Ok(body)
}

/// Build a richer PR body by handing the branch context to the configured
/// LLM (`fledge ai use ...` / `--provider` / `--model`). Includes the full
/// commit log, file-level diffstat, and a truncated unified diff so the
/// model has enough context to write a real description, not just a list
/// of subjects.
fn generate_body_with_ai(
    branch: &str,
    base: &str,
    provider_override: Option<&str>,
    model_override: Option<&str>,
    json: bool,
) -> Result<String> {
    let range = format!("{base}..{branch}");
    let commits =
        git_output(&["log", "--pretty=format:%h %s%n%b%n---", &range]).unwrap_or_default();
    let diffstat = git_output(&["diff", "--stat", &range]).unwrap_or_default();

    // Truncate the diff to keep small/local models inside their context
    // window. 600 lines is enough for most PRs to convey shape; reviewers
    // see the full diff on GitHub regardless.
    let diff_full = git_output(&["diff", &range]).unwrap_or_default();
    let mut diff_lines: Vec<&str> = diff_full.lines().collect();
    let truncated = diff_lines.len() > 600;
    if truncated {
        diff_lines.truncate(600);
    }
    let diff = diff_lines.join("\n");
    let truncation_note = if truncated {
        "\n\n[diff truncated to 600 lines for context]"
    } else {
        ""
    };

    let prompt = format!(
        "You are writing a GitHub pull request description.\n\
         \n\
         Branch: {branch} → {base}\n\
         \n\
         Commits:\n\
         {commits}\n\
         \n\
         Files changed:\n\
         {diffstat}\n\
         \n\
         Diff:\n\
         {diff}{truncation_note}\n\
         \n\
         Write a Markdown PR description with:\n\
         \n\
         ## Summary\n\
         - 2 to 5 bullets describing what changed and why\n\
         \n\
         ## Test plan\n\
         - [ ] checklist items the reviewer should verify\n\
         \n\
         Be concrete. Reference specific files or functions when relevant. \
         Do not invent features that aren't in the diff. \
         Do not include any preamble or sign-off — output ONLY the Markdown body."
    );

    let config = crate::config::Config::load().context("loading config")?;
    let provider = crate::llm::build_provider(
        &config,
        &crate::llm::ProviderOverride {
            provider: provider_override.map(|s| s.to_string()),
            model: model_override.map(|s| s.to_string()),
        },
    )?;

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start(&format!(
            "Drafting PR body [{}]:",
            crate::llm::describe(&*provider)
        )))
    };
    let answer = provider.invoke(&prompt);
    if let Some(sp) = sp {
        sp.finish();
    }
    Ok(answer?.trim().to_string())
}

fn format_commit_subject_as_bullet(subject: &str) -> String {
    // Strip a leading conventional-commit prefix like "feat: ", "fix(scope): ".
    let stripped = match subject.find(": ") {
        Some(idx) => {
            let prefix = &subject[..idx];
            let looks_like_type = prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '(' || c == ')' || c == '-' || c == '_');
            if looks_like_type && !prefix.is_empty() {
                subject[idx + 2..].trim()
            } else {
                subject
            }
        }
        None => subject,
    };

    let mut chars = stripped.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => stripped.to_string(),
    }
}

fn print_pr_preview(title: &str, body: &str, head: &str, base: &str, draft: bool) {
    let bar = style("─".repeat(60)).dim();
    println!();
    println!("{}", bar);
    println!("{} {}", style("Title:").bold(), style(title).green());
    println!(
        "{}  {} → {}{}",
        style("Branch:").bold(),
        style(head).cyan(),
        style(base).dim(),
        if draft {
            style(" (draft)").yellow().to_string()
        } else {
            String::new()
        },
    );
    println!();
    for line in body.lines() {
        println!("  {}", line);
    }
    if !body.ends_with('\n') {
        println!();
    }
    println!("{}", bar);
}

pub fn generate_title_from_branch(branch: &str) -> String {
    let name = VALID_BRANCH_TYPES
        .iter()
        .find_map(|t| branch.strip_prefix(&format!("{t}/")))
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
pub fn build_branch_name(
    name: &str,
    branch_type: &str,
    issue: Option<u64>,
    prefix: Option<&str>,
    config: &WorkConfig,
) -> String {
    let sanitized = sanitize_branch_name(name);
    if let Some(pfx) = prefix {
        format!("{}/{sanitized}", pfx.trim_end_matches('/'))
    } else {
        let author = Config::load()
            .ok()
            .and_then(|c| c.author_or_git())
            .map(|a| sanitize_branch_name(&a))
            .unwrap_or_default();
        let name_part = match issue {
            Some(num) => format!("{num}-{sanitized}"),
            None => sanitized,
        };
        config
            .branch_format
            .replace("{author}", &author)
            .replace("{type}", branch_type)
            .replace("{issue}", &issue.map(|n| n.to_string()).unwrap_or_default())
            .replace("{name}", &name_part)
    }
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

    #[test]
    fn test_generate_title_docs_prefix() {
        assert_eq!(
            generate_title_from_branch("docs/update-readme"),
            "Update readme"
        );
    }

    #[test]
    fn test_generate_title_hotfix_prefix() {
        assert_eq!(
            generate_title_from_branch("hotfix/critical-bug"),
            "Critical bug"
        );
    }

    // Branch name building tests

    #[test]
    fn test_build_branch_default_feat() {
        let config = WorkConfig {
            branch_format: "{type}/{name}".to_string(),
            default_type: "feat".to_string(),
            branch_types: None,
        };
        assert_eq!(
            build_branch_name("login-page", "feat", None, None, &config),
            "feat/login-page"
        );
    }

    #[test]
    fn test_build_branch_fix_type() {
        let config = WorkConfig {
            branch_format: "{type}/{name}".to_string(),
            default_type: "feat".to_string(),
            branch_types: None,
        };
        assert_eq!(
            build_branch_name("login-crash", "fix", None, None, &config),
            "fix/login-crash"
        );
    }

    #[test]
    fn test_build_branch_with_issue() {
        let config = WorkConfig {
            branch_format: "{type}/{name}".to_string(),
            default_type: "feat".to_string(),
            branch_types: None,
        };
        assert_eq!(
            build_branch_name("login-crash", "fix", Some(42), None, &config),
            "fix/42-login-crash"
        );
    }

    #[test]
    fn test_build_branch_with_prefix() {
        let config = WorkConfig::default();
        assert_eq!(
            build_branch_name("search", "feat", None, Some("user/leif"), &config),
            "user/leif/search"
        );
    }

    #[test]
    fn test_build_branch_prefix_trailing_slash() {
        let config = WorkConfig::default();
        assert_eq!(
            build_branch_name("search", "feat", None, Some("user/leif/"), &config),
            "user/leif/search"
        );
    }

    #[test]
    fn test_build_branch_custom_format() {
        let config = WorkConfig {
            branch_format: "user/leif/{type}/{name}".to_string(),
            default_type: "feat".to_string(),
            branch_types: None,
        };
        assert_eq!(
            build_branch_name("search", "feat", None, None, &config),
            "user/leif/feat/search"
        );
    }

    #[test]
    fn test_build_branch_issue_format() {
        let config = WorkConfig {
            branch_format: "{type}/{issue}-{name}".to_string(),
            default_type: "feat".to_string(),
            branch_types: None,
        };
        assert_eq!(
            build_branch_name("login-crash", "fix", Some(42), None, &config),
            "fix/42-42-login-crash"
        );
    }

    #[test]
    fn test_build_branch_issue_in_format_no_issue() {
        let config = WorkConfig {
            branch_format: "{type}/{name}".to_string(),
            default_type: "feat".to_string(),
            branch_types: None,
        };
        assert_eq!(
            build_branch_name("search", "feat", None, None, &config),
            "feat/search"
        );
    }

    #[test]
    fn test_work_config_defaults() {
        let config = WorkConfig::default();
        assert_eq!(config.branch_format, "{author}/{type}/{name}");
        assert_eq!(config.default_type, "feat");
    }

    #[test]
    fn test_work_config_from_toml() {
        let toml_str = r#"
[work]
branch_format = "{type}/PROJ-{issue}-{name}"
default_type = "fix"
"#;
        let file: FledgeWorkFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.work.branch_format, "{type}/PROJ-{issue}-{name}");
        assert_eq!(file.work.default_type, "fix");
    }

    #[test]
    fn test_work_config_partial_toml() {
        let toml_str = r#"
[work]
default_type = "chore"
"#;
        let file: FledgeWorkFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.work.branch_format, "{author}/{type}/{name}");
        assert_eq!(file.work.default_type, "chore");
    }

    #[test]
    fn test_work_config_missing_section() {
        let toml_str = r#"
[tasks]
test = "cargo test"
"#;
        let file: FledgeWorkFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.work.branch_format, "{author}/{type}/{name}");
        assert_eq!(file.work.default_type, "feat");
    }

    #[test]
    fn test_valid_branch_types() {
        assert!(VALID_BRANCH_TYPES.contains(&"feat"));
        assert!(VALID_BRANCH_TYPES.contains(&"feature"));
        assert!(VALID_BRANCH_TYPES.contains(&"fix"));
        assert!(VALID_BRANCH_TYPES.contains(&"bug"));
        assert!(VALID_BRANCH_TYPES.contains(&"chore"));
        assert!(VALID_BRANCH_TYPES.contains(&"task"));
        assert!(VALID_BRANCH_TYPES.contains(&"docs"));
        assert!(VALID_BRANCH_TYPES.contains(&"hotfix"));
        assert!(VALID_BRANCH_TYPES.contains(&"refactor"));
        assert!(!VALID_BRANCH_TYPES.contains(&"yolo"));
    }

    #[test]
    fn extract_pr_number_basic() {
        assert_eq!(
            extract_pr_number("https://github.com/owner/repo/pull/42"),
            Some(42)
        );
    }

    #[test]
    fn extract_pr_number_trailing_slash() {
        assert_eq!(
            extract_pr_number("https://github.com/owner/repo/pull/42/"),
            Some(42)
        );
    }

    #[test]
    fn extract_pr_number_with_query() {
        assert_eq!(
            extract_pr_number("https://github.com/owner/repo/pull/42?q=1"),
            Some(42)
        );
    }

    #[test]
    fn extract_pr_number_with_fragment() {
        assert_eq!(
            extract_pr_number("https://github.com/owner/repo/pull/42#comment"),
            Some(42)
        );
    }

    #[test]
    fn extract_pr_number_subpath() {
        assert_eq!(
            extract_pr_number("https://github.com/owner/repo/pull/42/files"),
            Some(42)
        );
    }

    #[test]
    fn format_commit_strips_feat_prefix() {
        assert_eq!(
            format_commit_subject_as_bullet("feat: add search command"),
            "Add search command"
        );
    }

    #[test]
    fn format_commit_strips_scoped_prefix() {
        assert_eq!(
            format_commit_subject_as_bullet("fix(work): null pointer on empty branch"),
            "Null pointer on empty branch"
        );
    }

    #[test]
    fn format_commit_uppercases_first_letter_with_no_prefix() {
        assert_eq!(
            format_commit_subject_as_bullet("update readme"),
            "Update readme"
        );
    }

    #[test]
    fn format_commit_leaves_already_capitalized_alone() {
        assert_eq!(
            format_commit_subject_as_bullet("Refactor work module"),
            "Refactor work module"
        );
    }

    #[test]
    fn format_commit_does_not_strip_unrelated_colons() {
        // A subject like "Note: this is fine" should NOT be treated as a
        // conventional-commit prefix — "Note" matches the alphanumeric rule
        // but is real prose. The current heuristic treats it as a prefix and
        // strips it; document that behavior so future changes are deliberate.
        assert_eq!(
            format_commit_subject_as_bullet("Note: something"),
            "Something"
        );
    }

    #[test]
    fn extract_pr_number_no_pull_segment() {
        assert_eq!(
            extract_pr_number("https://github.com/owner/repo/issues/42"),
            None
        );
    }

    #[test]
    fn extract_pr_number_not_a_url() {
        assert_eq!(extract_pr_number("not-a-url"), None);
    }
}
