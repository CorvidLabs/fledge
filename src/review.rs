use anyhow::{Result, bail};
use console::style;
use std::process::Command;

pub struct ReviewOptions {
    pub base: Option<String>,
    pub file: Option<String>,
}

pub fn run(options: ReviewOptions) -> Result<()> {
    ensure_claude_cli()?;
    ensure_git_repo()?;

    let base = match options.base {
        Some(b) => b,
        None => default_branch()?,
    };

    let diff = get_diff(&base, options.file.as_deref())?;

    if diff.is_empty() {
        bail!("No changes to review against '{}'.", base);
    }

    let diff_stats = get_diff_stats(&base, options.file.as_deref())?;

    println!(
        "{} Reviewing changes against {} ...\n",
        style("●").cyan().bold(),
        style(&base).cyan()
    );

    if !diff_stats.is_empty() {
        println!("{}\n", style(&diff_stats).dim());
    }

    let prompt = format!(
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
        ```diff\n{}\n```",
        diff
    );

    let status = Command::new("claude").args(["--print", &prompt]).status()?;

    if !status.success() {
        bail!("claude CLI exited with an error.");
    }

    Ok(())
}

fn ensure_claude_cli() -> Result<()> {
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

fn ensure_git_repo() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !output.status.success() {
        bail!("Not a git repository.");
    }
    Ok(())
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
