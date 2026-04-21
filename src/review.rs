use anyhow::{bail, Result};
use console::style;
use std::process::Command;

pub struct ReviewOptions {
    pub base: Option<String>,
    pub file: Option<String>,
    pub json: bool,
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

    let prompt = build_prompt(&diff);

    let sp = crate::spinner::Spinner::start(&format!("Reviewing changes against {}:", &base));

    let output = Command::new("claude").args(["--print", &prompt]).output()?;

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

fn build_prompt(diff: &str) -> String {
    format!(
        "You are a senior code reviewer. Review the following git diff and provide actionable feedback.\n\
        Focus on:\n\
        - Bugs and logic errors\n\
        - Security issues\n\
        - Performance concerns\n\
        - Code clarity and maintainability\n\
        \n\
        Be concise. Use markdown formatting. Only comment on things worth changing.\n\
        If the code looks good, say so briefly.\n\
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
        let prompt = build_prompt("+ added line\n- removed line");
        assert!(prompt.contains("+ added line"));
        assert!(prompt.contains("- removed line"));
        assert!(prompt.contains("```diff"));
    }

    #[test]
    fn build_prompt_includes_review_criteria() {
        let prompt = build_prompt("some diff");
        assert!(prompt.contains("Bugs and logic errors"));
        assert!(prompt.contains("Security issues"));
        assert!(prompt.contains("Performance concerns"));
        assert!(prompt.contains("Code clarity"));
    }

    #[test]
    fn review_options_defaults() {
        let opts = ReviewOptions {
            base: None,
            file: None,
            json: false,
        };
        assert!(opts.base.is_none());
        assert!(opts.file.is_none());
        assert!(!opts.json);
    }

    #[test]
    fn review_options_with_base() {
        let opts = ReviewOptions {
            base: Some("develop".to_string()),
            file: None,
            json: true,
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
        };
        assert_eq!(opts.file.unwrap(), "src/main.rs");
    }
}
