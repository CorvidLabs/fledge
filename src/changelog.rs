use anyhow::{Context, Result};
use console::style;
use std::process::Command;

pub struct ChangelogOptions {
    pub limit: usize,
    pub json: bool,
    pub tag: Option<String>,
    pub unreleased: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Release {
    tag: String,
    date: String,
    sections: Vec<Section>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Section {
    kind: String,
    commits: Vec<CommitEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CommitEntry {
    hash: String,
    message: String,
}

pub fn run(opts: ChangelogOptions) -> Result<()> {
    let tags = list_tags()?;

    if tags.is_empty() {
        println!(
            "{} No tags found. Tag a release first: {}",
            style("*").cyan().bold(),
            style("git tag v0.1.0").dim()
        );
        return Ok(());
    }

    let releases = if let Some(ref tag) = opts.tag {
        let idx = tags.iter().position(|t| t.0 == *tag);
        match idx {
            Some(i) => {
                let prev = if i + 1 < tags.len() {
                    Some(tags[i + 1].0.as_str())
                } else {
                    None
                };
                vec![build_release(&tags[i].0, &tags[i].1, prev)?]
            }
            None => anyhow::bail!("Tag '{}' not found", tag),
        }
    } else if opts.unreleased {
        let prev = tags.first().map(|t| t.0.as_str());
        vec![build_release("Unreleased", &current_date(), prev)?]
    } else {
        let mut releases = Vec::new();
        for (i, (tag, date)) in tags.iter().enumerate().take(opts.limit) {
            let prev = if i + 1 < tags.len() {
                Some(tags[i + 1].0.as_str())
            } else {
                None
            };
            releases.push(build_release(tag, date, prev)?);
        }
        releases
    };

    if opts.json {
        let envelope = serde_json::json!({
            "schema_version": 1,
            "action": "changelog",
            "releases": releases,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    for release in &releases {
        println!(
            "\n{} {}",
            style(&release.tag).green().bold(),
            style(format!("({})", release.date)).dim()
        );

        if release.sections.is_empty() {
            println!("  {}", style("No conventional commits found").dim());
            continue;
        }

        for section in &release.sections {
            println!("\n  {}:", style(&section.kind).cyan().bold());
            for commit in &section.commits {
                println!(
                    "    {} {}",
                    style(&commit.hash.chars().take(7).collect::<String>()).dim(),
                    commit.message
                );
            }
        }
    }

    println!();
    Ok(())
}

fn list_tags() -> Result<Vec<(String, String)>> {
    let output = Command::new("git")
        .args([
            "tag",
            "--sort=-version:refname",
            "--format=%(refname:short)\t%(creatordate:short)",
        ])
        .output()
        .context("running git tag")?;

    if !output.status.success() {
        anyhow::bail!(
            "git tag failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut parts = line.splitn(2, '\t');
            let tag = parts.next().unwrap_or("").to_string();
            let date = parts.next().unwrap_or("").to_string();
            (tag, date)
        })
        .collect())
}

fn commits_between(from: Option<&str>, to: &str) -> Result<Vec<(String, String)>> {
    let range = match from {
        Some(prev) => format!("{prev}..{to}"),
        None => to.to_string(),
    };

    let output = Command::new("git")
        .args(["log", &range, "--pretty=format:%h\t%s", "--no-merges"])
        .output()
        .context("running git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "  {} git log for range '{}' failed: {}",
            style("⚠").yellow().bold(),
            range,
            stderr.trim()
        );
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut parts = line.splitn(2, '\t');
            let hash = parts.next().unwrap_or("").to_string();
            let msg = parts.next().unwrap_or("").to_string();
            (hash, msg)
        })
        .collect())
}

fn build_release(tag: &str, date: &str, prev: Option<&str>) -> Result<Release> {
    let to_ref = if tag == "Unreleased" { "HEAD" } else { tag };
    let raw_commits = commits_between(prev, to_ref)?;

    let mut groups: std::collections::BTreeMap<String, Vec<CommitEntry>> =
        std::collections::BTreeMap::new();

    for (hash, msg) in raw_commits {
        let (kind, message) = classify_commit(&msg);
        groups
            .entry(kind)
            .or_default()
            .push(CommitEntry { hash, message });
    }

    let sections: Vec<Section> = groups
        .into_iter()
        .map(|(kind, commits)| Section { kind, commits })
        .collect();

    Ok(Release {
        tag: tag.to_string(),
        date: date.to_string(),
        sections,
    })
}

fn classify_commit(msg: &str) -> (String, String) {
    let prefixes = [
        ("feat", "Features"),
        ("fix", "Fixes"),
        ("docs", "Documentation"),
        ("style", "Style"),
        ("refactor", "Refactoring"),
        ("perf", "Performance"),
        ("test", "Tests"),
        ("build", "Build"),
        ("ci", "CI"),
        ("chore", "Chores"),
    ];

    for (prefix, label) in &prefixes {
        if let Some(rest) = msg.strip_prefix(prefix) {
            if let Some(rest) = rest.strip_prefix(':') {
                return (label.to_string(), rest.trim().to_string());
            }
            if let Some(rest) = rest.strip_prefix('(') {
                if let Some(after_scope) = rest.find("):") {
                    return (
                        label.to_string(),
                        rest[after_scope + 2..].trim().to_string(),
                    );
                }
            }
        }
    }

    ("Other".to_string(), msg.to_string())
}

fn current_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_feat() {
        let (kind, msg) = classify_commit("feat: add changelog command");
        assert_eq!(kind, "Features");
        assert_eq!(msg, "add changelog command");
    }

    #[test]
    fn classify_fix_with_scope() {
        let (kind, msg) = classify_commit("fix(parser): handle empty input");
        assert_eq!(kind, "Fixes");
        assert_eq!(msg, "handle empty input");
    }

    #[test]
    fn classify_unknown() {
        let (kind, msg) = classify_commit("update readme");
        assert_eq!(kind, "Other");
        assert_eq!(msg, "update readme");
    }

    #[test]
    fn classify_docs() {
        let (kind, msg) = classify_commit("docs: update API reference");
        assert_eq!(kind, "Documentation");
        assert_eq!(msg, "update API reference");
    }

    #[test]
    fn classify_all_prefixes() {
        let cases = vec![
            ("feat: x", "Features"),
            ("fix: x", "Fixes"),
            ("docs: x", "Documentation"),
            ("style: x", "Style"),
            ("refactor: x", "Refactoring"),
            ("perf: x", "Performance"),
            ("test: x", "Tests"),
            ("build: x", "Build"),
            ("ci: x", "CI"),
            ("chore: x", "Chores"),
        ];
        for (input, expected_kind) in cases {
            let (kind, _) = classify_commit(input);
            assert_eq!(kind, expected_kind, "failed for input: {input}");
        }
    }

    #[test]
    fn classify_no_space_after_colon() {
        let (kind, msg) = classify_commit("feat:no space");
        assert_eq!(kind, "Features");
        assert_eq!(msg, "no space");
    }

    #[test]
    fn classify_scope_no_space_after_paren() {
        let (kind, msg) = classify_commit("fix(core):handle edge case");
        assert_eq!(kind, "Fixes");
        assert_eq!(msg, "handle edge case");
    }
}
