use anyhow::{bail, Result};
use console::style;
use std::fmt;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub enum ReviewFormat {
    #[default]
    Summary,
    Checklist,
    Inline,
}

impl fmt::Display for ReviewFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewFormat::Summary => write!(f, "summary"),
            ReviewFormat::Checklist => write!(f, "checklist"),
            ReviewFormat::Inline => write!(f, "inline"),
        }
    }
}

impl std::str::FromStr for ReviewFormat {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "summary" => Ok(ReviewFormat::Summary),
            "checklist" => Ok(ReviewFormat::Checklist),
            "inline" => Ok(ReviewFormat::Inline),
            other => Err(format!(
                "unknown review format '{}' (expected: summary, checklist, inline)",
                other
            )),
        }
    }
}

pub struct ReviewOptions {
    pub base: Option<String>,
    pub file: Option<String>,
    pub json: bool,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub format: ReviewFormat,
}

pub fn run(options: ReviewOptions) -> Result<()> {
    crate::github::ensure_claude_cli()?;
    crate::github::ensure_git_repo()?;

    let base = match options.base {
        Some(b) => b,
        None => default_branch()?,
    };

    let diff = get_diff(&base, options.file.as_deref())?;

    if diff.is_empty() {
        bail!("No changes to review against '{}'.", base);
    }

    let diff_stats = get_diff_stats(&base, options.file.as_deref())?;

    if !options.json && !diff_stats.is_empty() {
        println!("{}\n", style(&diff_stats).dim());
    }

    let prompt = build_prompt(&diff, &options.format, options.prompt.as_deref());

    let sp = crate::spinner::Spinner::start(&format!("Reviewing changes against {}:", &base));

    let mut args = Vec::new();
    if let Some(ref model) = options.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }
    args.push("--print".to_string());
    args.push(prompt);

    let output = Command::new("claude").args(&args).output()?;

    sp.finish();
    println!();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            eprintln!("{stderr}");
        }
        bail!("claude CLI exited with an error.");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    if options.json {
        let response = serde_json::json!({
            "base": base,
            "file": options.file,
            "diff_stats": diff_stats,
            "review": stdout.trim(),
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print!("{stdout}");
    }

    Ok(())
}

fn build_prompt(diff: &str, format: &ReviewFormat, custom_prompt: Option<&str>) -> String {
    let format_instruction = match format {
        ReviewFormat::Summary => {
            "Be concise. Use markdown formatting. Only comment on things worth changing.\n\
             If the code looks good, say so briefly."
                .to_string()
        }
        ReviewFormat::Checklist => {
            "Format your review as a markdown checklist with - [ ] for issues found and - [x] for areas that look good."
                .to_string()
        }
        ReviewFormat::Inline => {
            "For each file in the diff, provide inline comments in the format: `file:line - comment`. Group by file."
                .to_string()
        }
    };

    let custom_section = match custom_prompt {
        Some(p) => format!("\n\nAdditional review focus: {p}"),
        None => String::new(),
    };

    format!(
        "You are a senior code reviewer. Review the following git diff and provide actionable feedback.\n\
        Focus on:\n\
        - Bugs and logic errors\n\
        - Security issues\n\
        - Performance concerns\n\
        - Code clarity and maintainability\n\
        \n\
        {format_instruction}{custom_section}\n\
        \n\
        ```diff\n{diff}\n```"
    )
}

fn default_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(name) = branch.strip_prefix("origin/") {
            return Ok(name.to_string());
        }
        return Ok(branch);
    }

    for candidate in &["main", "master"] {
        let check = Command::new("git")
            .args(["rev-parse", "--verify", candidate])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        if check.success() {
            return Ok(candidate.to_string());
        }
    }

    Ok("main".to_string())
}

fn get_diff(base: &str, file: Option<&str>) -> Result<String> {
    let mut args = vec!["diff", base];
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }

    let output = Command::new("git").args(&args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn get_diff_stats(base: &str, file: Option<&str>) -> Result<String> {
    let mut args = vec!["diff", "--stat", base];
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }

    let output = Command::new("git").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_contains_diff() {
        let prompt = build_prompt("+ added line\n- removed line", &ReviewFormat::Summary, None);
        assert!(prompt.contains("+ added line"));
        assert!(prompt.contains("- removed line"));
        assert!(prompt.contains("```diff"));
    }

    #[test]
    fn build_prompt_includes_review_criteria() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None);
        assert!(prompt.contains("Bugs and logic errors"));
        assert!(prompt.contains("Security issues"));
        assert!(prompt.contains("Performance concerns"));
        assert!(prompt.contains("Code clarity"));
    }

    #[test]
    fn build_prompt_summary_format() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None);
        assert!(prompt.contains("Be concise"));
        assert!(prompt.contains("Use markdown formatting"));
    }

    #[test]
    fn build_prompt_checklist_format() {
        let prompt = build_prompt("some diff", &ReviewFormat::Checklist, None);
        assert!(prompt.contains("markdown checklist"));
        assert!(prompt.contains("- [ ]"));
        assert!(prompt.contains("- [x]"));
    }

    #[test]
    fn build_prompt_inline_format() {
        let prompt = build_prompt("some diff", &ReviewFormat::Inline, None);
        assert!(prompt.contains("file:line - comment"));
        assert!(prompt.contains("Group by file"));
    }

    #[test]
    fn build_prompt_with_custom_prompt() {
        let prompt = build_prompt(
            "some diff",
            &ReviewFormat::Summary,
            Some("Focus on security vulnerabilities"),
        );
        assert!(prompt.contains("Additional review focus: Focus on security vulnerabilities"));
    }

    #[test]
    fn build_prompt_without_custom_prompt() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None);
        assert!(!prompt.contains("Additional review focus"));
    }

    #[test]
    fn build_prompt_custom_prompt_with_checklist_format() {
        let prompt = build_prompt(
            "some diff",
            &ReviewFormat::Checklist,
            Some("Check for performance issues"),
        );
        assert!(prompt.contains("markdown checklist"));
        assert!(prompt.contains("Additional review focus: Check for performance issues"));
    }

    #[test]
    fn review_format_from_str() {
        assert!(matches!(
            "summary".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Summary
        ));
        assert!(matches!(
            "checklist".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Checklist
        ));
        assert!(matches!(
            "inline".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Inline
        ));
        assert!(matches!(
            "SUMMARY".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Summary
        ));
        assert!("unknown".parse::<ReviewFormat>().is_err());
    }

    #[test]
    fn review_format_display() {
        assert_eq!(ReviewFormat::Summary.to_string(), "summary");
        assert_eq!(ReviewFormat::Checklist.to_string(), "checklist");
        assert_eq!(ReviewFormat::Inline.to_string(), "inline");
    }

    #[test]
    fn review_options_defaults() {
        let opts = ReviewOptions {
            base: None,
            file: None,
            json: false,
            model: None,
            prompt: None,
            format: ReviewFormat::Summary,
        };
        assert!(opts.base.is_none());
        assert!(opts.file.is_none());
        assert!(!opts.json);
        assert!(opts.model.is_none());
        assert!(opts.prompt.is_none());
        assert!(matches!(opts.format, ReviewFormat::Summary));
    }

    #[test]
    fn review_options_with_base() {
        let opts = ReviewOptions {
            base: Some("develop".to_string()),
            file: None,
            json: true,
            model: None,
            prompt: None,
            format: ReviewFormat::Summary,
        };
        assert_eq!(opts.base.unwrap(), "develop");
        assert!(opts.json);
    }

    #[test]
    fn review_options_with_file() {
        let opts = ReviewOptions {
            base: None,
            file: Some("src/main.rs".to_string()),
            json: false,
            model: None,
            prompt: None,
            format: ReviewFormat::Summary,
        };
        assert_eq!(opts.file.unwrap(), "src/main.rs");
    }

    #[test]
    fn review_options_with_all_new_fields() {
        let opts = ReviewOptions {
            base: None,
            file: None,
            json: false,
            model: Some("opus".to_string()),
            prompt: Some("Focus on security".to_string()),
            format: ReviewFormat::Checklist,
        };
        assert_eq!(opts.model.unwrap(), "opus");
        assert_eq!(opts.prompt.unwrap(), "Focus on security");
        assert!(matches!(opts.format, ReviewFormat::Checklist));
    }
}
