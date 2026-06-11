use anyhow::{Context, Result};
use console::style;
use std::path::Path;
use std::process::Command;

use crate::versioning::Version;

pub(super) fn generate_changelog_entry(dir: &Path, version: &Version, quiet: bool) -> Result<bool> {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let tag_name = format!("v{version}");

    let prev_tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let range = match &prev_tag {
        Some(tag) => format!("{tag}..HEAD"),
        None => "HEAD".to_string(),
    };

    let output = Command::new("git")
        .args(["log", &range, "--pretty=format:%h\t%s", "--no-merges"])
        .current_dir(dir)
        .output()
        .context("running git log for changelog")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<(&str, &str)> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| line.split_once('\t'))
        .collect();

    if commits.is_empty() {
        if !quiet {
            println!(
                "  {} No commits since last tag, skipping changelog",
                style("*").cyan().bold()
            );
        }
        return Ok(false);
    }

    let mut sections: std::collections::BTreeMap<&str, Vec<(&str, &str)>> =
        std::collections::BTreeMap::new();

    for (hash, msg) in &commits {
        let kind = classify_for_changelog(msg);
        sections.entry(kind).or_default().push((hash, msg));
    }

    let mut entry = format!("## [{tag_name}] - {date}\n\n");
    for (kind, items) in &sections {
        entry.push_str(&format!("### {kind}\n\n"));
        for (hash, msg) in items {
            let clean_msg = strip_conventional_prefix(msg);
            entry.push_str(&format!("- {clean_msg} ({hash})\n"));
        }
        entry.push('\n');
    }

    let changelog_path = dir.join("CHANGELOG.md");
    if changelog_path.exists() {
        let existing = std::fs::read_to_string(&changelog_path)?;
        if let Some(pos) = existing.find("\n## ") {
            let mut updated = String::new();
            updated.push_str(&existing[..pos + 1]);
            updated.push_str(&entry);
            updated.push_str(&existing[pos + 1..]);
            std::fs::write(&changelog_path, updated)?;
        } else {
            let mut updated = existing;
            updated.push('\n');
            updated.push_str(&entry);
            std::fs::write(&changelog_path, updated)?;
        }
    } else {
        let mut content = String::from("# Changelog\n\n");
        content.push_str(&entry);
        std::fs::write(&changelog_path, content)?;
    }

    if !quiet {
        println!("  Updated {}", style("CHANGELOG.md").cyan());
    }
    Ok(true)
}

pub(super) fn classify_for_changelog(msg: &str) -> &'static str {
    let prefixes = [
        ("feat", "Features"),
        ("fix", "Fixes"),
        ("docs", "Documentation"),
        ("perf", "Performance"),
        ("refactor", "Refactoring"),
        ("test", "Tests"),
        ("build", "Build"),
        ("ci", "CI"),
        ("chore", "Chores"),
        ("style", "Style"),
        // CorvidLabs-style prefixes (`Add:`, `Update:`, `Remove:`) — matched
        // case-insensitively like every other prefix.
        ("add", "Features"),
        ("update", "Changes"),
        ("remove", "Removals"),
    ];

    for (prefix, label) in &prefixes {
        let Some(head) = msg.get(..prefix.len()) else {
            continue;
        };
        if head.eq_ignore_ascii_case(prefix) {
            // Optional breaking-change marker: `type!:`.
            let rest = &msg[prefix.len()..];
            let rest = rest.strip_prefix('!').unwrap_or(rest);
            if rest.starts_with([':', '(']) {
                return label;
            }
        }
    }

    "Other"
}

pub(super) fn strip_conventional_prefix(msg: &str) -> &str {
    if let Some(colon_pos) = msg.find(':') {
        let prefix = &msg[..colon_pos];
        let after = msg[colon_pos + 1..].trim_start();
        let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
        let base = if let Some(paren) = prefix.find('(') {
            &prefix[..paren]
        } else {
            prefix
        };
        let known = [
            "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
            "add", "update", "remove",
        ];
        if known.iter().any(|k| base.eq_ignore_ascii_case(k)) {
            return after;
        }
    }
    msg
}
