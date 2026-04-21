use anyhow::{Context, Result};
use console::style;

use crate::config::Config;
use crate::github;

pub struct ChecksOptions {
    pub branch: Option<String>,
    pub json: bool,
}

pub fn run(opts: ChecksOptions) -> Result<()> {
    let config = Config::load()?;
    let token = config.github_token();
    let (owner, repo) = github::detect_repo()?;

    let branch = match opts.branch {
        Some(b) => b,
        None => current_branch()?,
    };

    let ref_path = format!("/repos/{owner}/{repo}/commits/{branch}/check-runs");
    let data = github::github_api_get(&ref_path, token.as_deref(), &[])?;

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    let check_runs = data["check_runs"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    if check_runs.is_empty() {
        println!(
            "{} No CI checks found for branch {}",
            style("*").cyan().bold(),
            style(&branch).green()
        );
        return Ok(());
    }

    println!(
        "{} CI checks for {}:\n",
        style("*").cyan().bold(),
        style(&branch).green()
    );

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut pending = 0u32;

    let max_name_len = check_runs
        .iter()
        .map(|c| c["name"].as_str().unwrap_or("").len())
        .max()
        .unwrap_or(0);

    for check in check_runs {
        let name = check["name"].as_str().unwrap_or("unknown");
        let status = check["status"].as_str().unwrap_or("unknown");
        let conclusion = check["conclusion"].as_str();

        let (icon, display_text, display_style): (&str, String, &str) = match (status, conclusion) {
            ("completed", Some("success")) => {
                passed += 1;
                ("✅", "passed".into(), "green")
            }
            ("completed", Some("failure")) => {
                failed += 1;
                ("❌", "failed".into(), "red")
            }
            ("completed", Some("cancelled")) => {
                failed += 1;
                ("🚫", "cancelled".into(), "yellow")
            }
            ("completed", Some("skipped")) => {
                passed += 1;
                ("⏭️", "skipped".into(), "dim")
            }
            ("completed", Some(c)) => {
                pending += 1;
                ("?", c.to_string(), "yellow")
            }
            _ => {
                pending += 1;
                ("🔄", "running".into(), "yellow")
            }
        };

        let icon = match display_style {
            "green" => style(icon).green().bold(),
            "red" => style(icon).red().bold(),
            "dim" => style(icon).dim().bold(),
            _ => style(icon).yellow().bold(),
        };
        let display = match display_style {
            "green" => style(display_text).green(),
            "red" => style(display_text).red(),
            "dim" => style(display_text).dim(),
            _ => style(display_text).yellow(),
        };

        let duration = match (check["started_at"].as_str(), check["completed_at"].as_str()) {
            (Some(start), Some(end)) => format_duration(start, end),
            (Some(_), None) => "running...".to_string(),
            _ => String::new(),
        };

        println!(
            "  {} {:<width$}  {:<10}  {}",
            icon,
            name,
            display,
            style(duration).dim(),
            width = max_name_len
        );
    }

    println!();
    let total = passed + failed + pending;
    print!("  {} checks: ", total);
    if passed > 0 {
        print!("{} passed", style(passed).green());
    }
    if failed > 0 {
        if passed > 0 {
            print!(", ");
        }
        print!("{} failed", style(failed).red());
    }
    if pending > 0 {
        if passed > 0 || failed > 0 {
            print!(", ");
        }
        print!("{} pending", style(pending).yellow());
    }
    println!();

    Ok(())
}

fn current_branch() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("running git")?;

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        anyhow::bail!("Not on a branch (detached HEAD?). Use --branch to specify.");
    }
    Ok(branch)
}

fn format_duration(start: &str, end: &str) -> String {
    let Ok(s) = chrono::DateTime::parse_from_rfc3339(start) else {
        return String::new();
    };
    let Ok(e) = chrono::DateTime::parse_from_rfc3339(end) else {
        return String::new();
    };
    let secs = (e - s).num_seconds();
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_seconds() {
        let start = "2024-01-01T00:00:00Z";
        let end = "2024-01-01T00:00:45Z";
        assert_eq!(format_duration(start, end), "45s");
    }

    #[test]
    fn format_duration_minutes() {
        let start = "2024-01-01T00:00:00Z";
        let end = "2024-01-01T00:02:30Z";
        assert_eq!(format_duration(start, end), "2m 30s");
    }

    #[test]
    fn format_duration_invalid() {
        assert_eq!(format_duration("bad", "date"), "");
    }
}
