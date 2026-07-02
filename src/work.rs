use anyhow::{bail, Context, Result};
use console::style;
use serde::Deserialize;
use std::process::Command;

use crate::config::Config;

const VALID_BRANCH_TYPES: &[&str] = &[
    "feat", "feature", "fix", "bug", "chore", "task", "docs", "hotfix", "refactor",
];

/// Commit types recognized as an existing conventional-commit prefix on a
/// `-m` message: the valid branch types plus the remaining standard
/// conventional-commit types and the capitalized `Add:`/`Update:`/`Remove:`
/// style used across CorvidLabs repos. Matching is case-insensitive.
const CONVENTIONAL_COMMIT_TYPES: &[&str] = &[
    "feat", "feature", "fix", "bug", "chore", "task", "docs", "hotfix", "refactor", "style",
    "perf", "test", "build", "ci", "add", "update", "remove",
];

/// Per-command JSON schema versions for `work` subcommands. See lanes.rs for
/// rationale.
const WORK_START_SCHEMA: u32 = 1;
const WORK_COMMIT_SCHEMA: u32 = 1;
const WORK_PUSH_SCHEMA: u32 = 1;
const WORK_STATUS_SCHEMA: u32 = 2;

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
    Commit {
        message: Option<String>,
        commit_type: Option<String>,
        scope: Option<String>,
        all: bool,
        ai: bool,
        provider: Option<String>,
        model: Option<String>,
        json: bool,
    },
    Push {
        force: bool,
        json: bool,
    },
    Status {
        json: bool,
    },
    DeprecatedPr,
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
        WorkAction::Commit {
            message,
            commit_type,
            scope,
            all,
            ai,
            provider,
            model,
            json,
        } => commit(
            message.as_deref(),
            commit_type.as_deref(),
            scope.as_deref(),
            all,
            ai,
            provider.as_deref(),
            model.as_deref(),
            json,
        ),
        WorkAction::Push { force, json } => push(force, json),
        WorkAction::Status { json } => status(json),
        WorkAction::DeprecatedPr => {
            eprintln!(
                "{} `fledge work pr` has been removed.",
                console::style("⚠").yellow().bold()
            );
            eprintln!(
                "  Use `fledge github prs create` (fledge-plugin-github) to open a pull request."
            );
            eprintln!("  Or directly: `gh pr create` — https://cli.github.com/manual/gh_pr_create");
            std::process::exit(1);
        }
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
        let payload = crate::envelope::action(
            WORK_START_SCHEMA,
            "work_start",
            serde_json::json!({
                "branch": branch_name,
                "base": base_branch,
                "type": btype,
                "prefix": prefix,
                "issue": issue,
            }),
        );
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
fn commit(
    message: Option<&str>,
    commit_type: Option<&str>,
    scope: Option<&str>,
    all: bool,
    ai: bool,
    provider_override: Option<&str>,
    model_override: Option<&str>,
    json: bool,
) -> Result<()> {
    if let Some(s) = scope {
        crate::utils::validate_commit_scope(s)?;
    }

    let branch = current_branch()?;
    let config = load_work_config();

    let inferred_type = commit_type
        .map(|t| t.to_string())
        .or_else(|| {
            VALID_BRANCH_TYPES
                .iter()
                .find(|t| branch.starts_with(&format!("{t}/")))
                .map(|t| t.to_string())
        })
        .unwrap_or_else(|| config.default_type.clone());

    if all {
        git_output(&["add", "-A"])?;
    }

    let staged = git_output(&["diff", "--cached", "--name-only"])?;
    if staged.is_empty() {
        let unstaged = git_output(&["status", "--porcelain"])?;
        if unstaged.is_empty() {
            bail!("Nothing to commit — working tree is clean.");
        } else {
            bail!(
                "No staged changes. Stage files with `git add` or use `fledge work commit --all`."
            );
        }
    }

    let commit_msg = if ai {
        generate_commit_message_with_ai(
            &inferred_type,
            scope,
            provider_override,
            model_override,
            json,
        )?
    } else if let Some(msg) = message {
        build_commit_message(&inferred_type, scope, msg)
    } else {
        if !crate::utils::is_interactive() {
            bail!("No commit message provided. Use -m or --ai in non-interactive mode.");
        }
        let msg: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!("{}:", inferred_type))
            .interact_text()?;
        build_commit_message(&inferred_type, scope, &msg)
    };

    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git commit failed: {}", stderr.trim());
    }

    let hash = git_output(&["rev-parse", "--short", "HEAD"])?;

    if json {
        let payload = crate::envelope::action(
            WORK_COMMIT_SCHEMA,
            "work_commit",
            serde_json::json!({
                "hash": hash,
                "message": commit_msg,
                "branch": branch,
            }),
        );
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "{} Committed {} on {}",
            style("✅").green().bold(),
            style(&hash).yellow(),
            style(&branch).cyan()
        );
        println!("  {}", style(&commit_msg).dim());
    }

    Ok(())
}

fn generate_commit_message_with_ai(
    commit_type: &str,
    scope: Option<&str>,
    provider_override: Option<&str>,
    model_override: Option<&str>,
    json: bool,
) -> Result<String> {
    let diff = git_output(&["diff", "--cached"])?;
    if diff.is_empty() {
        bail!("No staged diff for AI to analyze.");
    }

    let mut diff_lines: Vec<&str> = diff.lines().collect();
    let truncated = diff_lines.len() > 400;
    if truncated {
        diff_lines.truncate(400);
    }
    let diff_text = diff_lines.join("\n");
    let truncation_note = if truncated {
        "\n\n[diff truncated to 400 lines]"
    } else {
        ""
    };

    let scope_instruction = match scope {
        Some(s) => format!("The scope is '{s}', so the format is: {commit_type}({s}): <message>"),
        None => format!("The format is: {commit_type}: <message>"),
    };

    let prompt = format!(
        "You are writing a git commit message in conventional-commit format.\n\
         \n\
         {scope_instruction}\n\
         \n\
         Staged diff:\n\
         {diff_text}{truncation_note}\n\
         \n\
         Write ONLY the commit message (one line). Be specific and concise. \
         Describe WHAT changed, not HOW. Do not include any explanation or preamble."
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
            "Generating commit message [{}]:",
            crate::llm::describe(&*provider)
        )))
    };
    let answer = provider.invoke(&prompt);
    if let Some(sp) = sp {
        sp.finish();
    }

    let raw = answer?.trim().to_string();
    let cleaned = raw
        .trim_start_matches('`')
        .trim_end_matches('`')
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim()
        .to_string();

    Ok(cleaned)
}

fn push(force: bool, json: bool) -> Result<()> {
    let branch = current_branch()?;
    let default = default_branch()?;

    if branch == default || branch.is_empty() {
        bail!(
            "Refusing to push the default branch '{}'. Switch to a feature branch first.",
            default
        );
    }

    // Check if there's anything to push
    let tracking = format!("origin/{branch}");
    let has_tracking = git_output(&["rev-parse", "--verify", &tracking]).is_ok();
    if has_tracking {
        let ahead = commits_ahead_of(&branch, &tracking)?;
        if ahead == 0 {
            bail!("No commits ahead of '{}'. Nothing to push.", tracking);
        }
    }

    crate::plugin::run_lifecycle_hook("pre_push")?;

    let sp = if json {
        None
    } else {
        Some(crate::spinner::Spinner::start(&format!(
            "Pushing {} to origin:",
            &branch
        )))
    };

    let mut args = vec!["push", "-u", "origin", &branch];
    if force {
        args.insert(1, "--force-with-lease");
    }

    let output = Command::new("git").args(&args).output()?;
    if let Some(sp) = sp {
        sp.finish();
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to push: {}", stderr.trim());
    }

    if json {
        let payload = crate::envelope::action(
            WORK_PUSH_SCHEMA,
            "work_push",
            serde_json::json!({
                "branch": branch,
                "remote": "origin",
                "force": force,
            }),
        );
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "{} Pushed {} to origin",
            style("✅").green().bold(),
            style(&branch).cyan()
        );
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
    let behind: Option<usize> = commits_behind_of(&branch, &default).ok();
    let dirty_count = git_output(&["status", "--porcelain"])?
        .lines()
        .filter(|l| !l.is_empty())
        .count();

    if json {
        let payload = crate::envelope::action(
            WORK_STATUS_SCHEMA,
            "work_status",
            serde_json::json!({
                "branch": branch,
                "default": default,
                "ahead": ahead,
                "behind": behind,
                "dirty": dirty_count,
            }),
        );
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        let behind_display = match behind {
            Some(n) if n > 0 => format!(", {} behind", n),
            _ => String::new(),
        };
        let dirty_display = if dirty_count > 0 {
            format!(
                "\n  Dirty: {} uncommitted {}",
                dirty_count,
                if dirty_count == 1 { "file" } else { "files" }
            )
        } else {
            String::new()
        };
        println!(
            "  Branch: {} ({} {} ahead of {}{}){dirty_display}",
            style(&branch).cyan(),
            ahead,
            if ahead == 1 { "commit" } else { "commits" },
            style(&default).dim(),
            behind_display,
        );
    }

    Ok(())
}

fn commits_behind_of(branch: &str, base: &str) -> Result<usize> {
    let range = format!("{branch}..{base}");
    let output = git_output(&["rev-list", "--count", &range])?;
    Ok(output.parse().unwrap_or(0))
}

fn commits_ahead_of(branch: &str, base: &str) -> Result<usize> {
    let range = format!("{base}..{branch}");
    let output = git_output(&["rev-list", "--count", &range])?;
    Ok(output.parse().unwrap_or(0))
}

/// Does the message already start with a conventional-commit prefix, i.e.
/// `type:`, `type(scope):`, or a breaking-change variant (`type!:`,
/// `type(scope)!:`) where `type` is a known commit type (case-insensitive)?
fn has_conventional_prefix(message: &str) -> bool {
    let Some(colon) = message.find(':') else {
        return false;
    };
    let head = &message[..colon];
    let head = head.strip_suffix('!').unwrap_or(head);
    let base = match head.find('(') {
        Some(open) if head.ends_with(')') => &head[..open],
        Some(_) => return false,
        None => head,
    };
    CONVENTIONAL_COMMIT_TYPES
        .iter()
        .any(|known| base.eq_ignore_ascii_case(known))
}

pub fn build_commit_message(commit_type: &str, scope: Option<&str>, message: &str) -> String {
    let trimmed = message.trim();
    // Already conventional-commit formatted — use it verbatim instead of
    // double-prefixing (e.g. `feat: feat: ...`).
    if has_conventional_prefix(trimmed) {
        return trimmed.to_string();
    }
    let mut chars = trimmed.chars();
    let msg = match chars.next() {
        Some(c) => c.to_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    };
    match scope {
        Some(s) if !s.is_empty() => format!("{}({}): {}", commit_type, s, msg),
        _ => format!("{}: {}", commit_type, msg),
    }
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
    fn test_build_commit_message_basic() {
        assert_eq!(
            build_commit_message("feat", None, "add search command"),
            "feat: add search command"
        );
    }

    #[test]
    fn test_build_commit_message_with_scope() {
        assert_eq!(
            build_commit_message("fix", Some("work"), "null pointer on empty branch"),
            "fix(work): null pointer on empty branch"
        );
    }

    #[test]
    fn test_build_commit_message_lowercases_first_char() {
        assert_eq!(
            build_commit_message("feat", None, "Add search"),
            "feat: add search"
        );
    }

    #[test]
    fn test_build_commit_message_empty_message() {
        assert_eq!(build_commit_message("chore", None, ""), "chore: ");
    }

    #[test]
    fn test_build_commit_message_with_whitespace() {
        assert_eq!(
            build_commit_message("feat", None, "  Add search  "),
            "feat: add search"
        );
        assert_eq!(
            build_commit_message("fix", Some("ui"), "\tFix padding\n"),
            "fix(ui): fix padding"
        );
    }

    #[test]
    fn test_build_commit_message_already_prefixed() {
        assert_eq!(
            build_commit_message("feat", None, "feat: note change"),
            "feat: note change"
        );
        assert_eq!(
            build_commit_message("feat", None, "fix: handle empty branch"),
            "fix: handle empty branch"
        );
    }

    #[test]
    fn test_build_commit_message_already_prefixed_with_scope() {
        assert_eq!(
            build_commit_message("feat", Some("cli"), "fix(parser): handle empty input"),
            "fix(parser): handle empty input"
        );
    }

    #[test]
    fn test_build_commit_message_already_prefixed_case_insensitive() {
        assert_eq!(
            build_commit_message("feat", None, "Fix: broken link"),
            "Fix: broken link"
        );
        assert_eq!(
            build_commit_message("fix", None, "Add: search command"),
            "Add: search command"
        );
        assert_eq!(
            build_commit_message("feat", None, "Update: dependency pins"),
            "Update: dependency pins"
        );
    }

    #[test]
    fn test_build_commit_message_already_prefixed_breaking() {
        assert_eq!(
            build_commit_message("feat", None, "feat!: drop legacy config"),
            "feat!: drop legacy config"
        );
        assert_eq!(
            build_commit_message("feat", None, "fix(core)!: change defaults"),
            "fix(core)!: change defaults"
        );
    }

    #[test]
    fn test_build_commit_message_non_type_colon_still_prefixed() {
        assert_eq!(
            build_commit_message("feat", None, "note: change"),
            "feat: note: change"
        );
        assert_eq!(
            build_commit_message("feat", None, "support http: and https: URLs"),
            "feat: support http: and https: URLs"
        );
    }
}
