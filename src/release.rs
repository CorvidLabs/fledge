use anyhow::{bail, Context, Result};
use console::style;
use regex_lite::Regex;
use std::path::Path;
use std::process::Command;

use crate::run::detect_project_type;
use crate::versioning::{parse_version, Version};

pub struct ReleaseOptions {
    pub bump: String,
    pub dry_run: bool,
    pub no_tag: bool,
    pub no_changelog: bool,
    /// Skip bumping any version files. Tag-only release, useful for repos
    /// whose version source-of-truth lives outside the working tree (e.g. a
    /// GitHub Release whose tag is the canonical version).
    pub no_bump: bool,
    pub push: bool,
    pub pre_lane: Option<String>,
    pub allow_dirty: bool,
    pub json: bool,
}

struct BumpResult {
    old: Version,
    new: Version,
    files_bumped: Vec<String>,
}

pub fn run(opts: ReleaseOptions) -> Result<()> {
    let dir = std::env::current_dir()?;

    preflight_checks(&dir, opts.allow_dirty)?;

    if let Some(ref lane) = opts.pre_lane {
        run_pre_lane(lane, opts.dry_run)?;
    }

    let new_version = resolve_target_version(&dir, &opts.bump)?;

    if opts.dry_run {
        let files_to_bump = if opts.no_bump {
            Vec::new()
        } else {
            detect_version_files(&dir)
        };

        if opts.json {
            let envelope = serde_json::json!({
                "schema_version": 1,
                "action": "release",
                "dry_run": true,
                "version": new_version.to_string(),
                "no_bump": opts.no_bump,
                "files_to_bump": files_to_bump,
                "will_changelog": !opts.no_changelog,
                "will_tag": !opts.no_tag,
                "will_push": opts.push,
                "tag": format!("v{}", new_version),
            });
            println!("{}", serde_json::to_string_pretty(&envelope)?);
            return Ok(());
        }

        println!(
            "{} Would release v{} (dry run)",
            style("*").cyan().bold(),
            new_version
        );
        if opts.no_bump {
            println!("  Tag-only release (--no-bump)");
        } else if files_to_bump.is_empty() {
            println!("  Tag-only release (no version files detected)");
        } else {
            for f in &files_to_bump {
                println!("  Would bump: {}", style(f).cyan());
            }
        }
        if !opts.no_changelog {
            println!("  Would update CHANGELOG.md");
        }
        if !opts.no_tag {
            println!("  Would create tag v{}", new_version);
        }
        if opts.push {
            println!("  Would push commit and tag");
        }
        return Ok(());
    }

    let result = if opts.no_bump {
        BumpResult {
            old: detect_current_version(&dir).unwrap_or(Version {
                major: 0,
                minor: 0,
                patch: 0,
            }),
            new: new_version.clone(),
            files_bumped: Vec::new(),
        }
    } else {
        bump_version_files(&dir, &new_version)?
    };

    if !opts.json {
        println!(
            "{} {} → {}",
            style("📦").bold(),
            style(&result.old).dim(),
            style(&result.new).green().bold()
        );

        for f in &result.files_bumped {
            println!("  Bumped {}", style(f).cyan());
        }

        if result.files_bumped.is_empty() {
            println!("  Tag-only release (no version files found)");
        }
    }

    let mut changelog_updated = false;
    if !opts.no_changelog {
        changelog_updated = generate_changelog_entry(&dir, &new_version, opts.json)?;
    }

    create_release_commit(
        &dir,
        &new_version,
        &result.files_bumped,
        !opts.no_changelog,
        opts.json,
    )?;

    if !opts.no_tag {
        create_tag(&dir, &new_version, opts.json)?;
    }

    if opts.push {
        push_release(&dir, &new_version, !opts.no_tag, opts.json)?;
    }

    if opts.json {
        let envelope = serde_json::json!({
            "schema_version": 1,
            "action": "release",
            "dry_run": false,
            "version": new_version.to_string(),
            "old_version": result.old.to_string(),
            "files_bumped": result.files_bumped,
            "changelog_updated": changelog_updated,
            "commit_created": true,
            "tag_created": !opts.no_tag,
            "tag": format!("v{}", new_version),
            "pushed": opts.push,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    println!(
        "\n{} Released v{}",
        style("✅").green().bold(),
        style(&new_version).green().bold()
    );

    if !opts.push {
        println!(
            "  Push with: {} && {}",
            style("git push").cyan(),
            style(format!("git push origin v{new_version}")).cyan()
        );
    }

    Ok(())
}

fn preflight_checks(dir: &Path, allow_dirty: bool) -> Result<()> {
    if !dir.join(".git").exists() {
        bail!("Not a git repository. Run `git init` first.");
    }

    if !allow_dirty {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dir)
            .output()
            .context("running git status")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            bail!(
                "Working tree is not clean. Commit or stash changes first, or use --allow-dirty.\n\n{}",
                stdout.trim()
            );
        }
    }

    Ok(())
}

fn run_pre_lane(lane: &str, dry_run: bool) -> Result<()> {
    println!(
        "{} Running pre-release lane: {}",
        style("🔄").bold(),
        style(lane).cyan()
    );

    let action = crate::lanes::LaneAction::Run {
        name: lane.to_string(),
        dry_run,
        json: false,
    };
    crate::lanes::run(action)?;

    println!("{} Pre-release lane passed", style("✅").green().bold());
    Ok(())
}

fn resolve_target_version(dir: &Path, bump: &str) -> Result<Version> {
    match bump {
        "major" | "minor" | "patch" => {
            let current = detect_current_version(dir)?;
            apply_bump(&current, bump)
        }
        _ => parse_version(bump),
    }
}

fn detect_current_version(dir: &Path) -> Result<Version> {
    // plugin.toml is the canonical fledge-ecosystem identity — prefer it over
    // any language-specific manifest. Rust plugins keep both Cargo.toml and
    // plugin.toml in sync; the plugin.toml version is the source of truth.
    if dir.join("plugin.toml").exists() {
        if let Ok(v) = read_plugin_toml_version(dir) {
            return parse_version(&v);
        }
    }

    let project_type = detect_project_type(dir);

    let version_str = match project_type {
        "rust" => read_cargo_version(dir)?,
        "node" => read_package_json_version(dir)?,
        "python" => read_python_version(dir)?,
        "ruby" => read_gemspec_version(dir)?,
        "java-gradle" => read_gradle_version(dir)?,
        "java-maven" => read_maven_version(dir)?,
        _ => read_version_from_tag(dir)?,
    };

    parse_version(&version_str)
}

fn read_cargo_version(dir: &Path) -> Result<String> {
    let content = std::fs::read_to_string(dir.join("Cargo.toml")).context("reading Cargo.toml")?;
    extract_toml_version(&content)
        .ok_or_else(|| anyhow::anyhow!("No version field found in Cargo.toml"))
}

/// Read `[plugin].version` from a fledge plugin manifest. Looks for the field
/// inside (or just after) the `[plugin]` table header so we don't accidentally
/// match a `version = "..."` line in a different table (e.g. a `[[commands]]`).
fn read_plugin_toml_version(dir: &Path) -> Result<String> {
    let content =
        std::fs::read_to_string(dir.join("plugin.toml")).context("reading plugin.toml")?;
    extract_versioned_toml_section(&content, "plugin")
        .ok_or_else(|| anyhow::anyhow!("No [plugin].version field found in plugin.toml"))
}

fn read_package_json_version(dir: &Path) -> Result<String> {
    let content =
        std::fs::read_to_string(dir.join("package.json")).context("reading package.json")?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    json["version"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No version field in package.json"))
}

fn read_python_version(dir: &Path) -> Result<String> {
    if let Ok(content) = std::fs::read_to_string(dir.join("pyproject.toml")) {
        if let Some(v) = extract_toml_version(&content) {
            return Ok(v);
        }
    }
    if let Ok(content) = std::fs::read_to_string(dir.join("setup.cfg")) {
        let re = Regex::new(r#"version\s*=\s*(\S+)"#).unwrap();
        if let Some(caps) = re.captures(&content) {
            return Ok(caps[1].to_string());
        }
    }
    read_version_from_tag(dir)
}

fn read_gemspec_version(dir: &Path) -> Result<String> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "gemspec") {
            let content = std::fs::read_to_string(&path)?;
            let re = Regex::new(r#"\.version\s*=\s*["']([^"']+)["']"#).unwrap();
            if let Some(caps) = re.captures(&content) {
                return Ok(caps[1].to_string());
            }
        }
    }
    read_version_from_tag(dir)
}

fn read_gradle_version(dir: &Path) -> Result<String> {
    for name in &["build.gradle.kts", "build.gradle"] {
        if let Ok(content) = std::fs::read_to_string(dir.join(name)) {
            let re = Regex::new(r#"version\s*=\s*["']([^"']+)["']"#).unwrap();
            if let Some(caps) = re.captures(&content) {
                return Ok(caps[1].to_string());
            }
        }
    }
    read_version_from_tag(dir)
}

fn read_maven_version(dir: &Path) -> Result<String> {
    let content = std::fs::read_to_string(dir.join("pom.xml")).context("reading pom.xml")?;
    let re = Regex::new(r"<version>([^<]+)</version>").unwrap();
    if let Some(caps) = re.captures(&content) {
        return Ok(caps[1].to_string());
    }
    read_version_from_tag(dir)
}

fn read_version_from_tag(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(dir)
        .output()
        .context("running git describe")?;

    if output.status.success() {
        let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let v = tag.strip_prefix('v').unwrap_or(&tag);
        return Ok(v.to_string());
    }

    bail!("No version found in project files or git tags. Specify an explicit version: fledge release 1.0.0")
}

fn extract_toml_version(content: &str) -> Option<String> {
    let re = Regex::new(r#"(?m)^version\s*=\s*"([^"]+)""#).unwrap();
    re.captures(content).map(|c| c[1].to_string())
}

/// Extract `version = "X.Y.Z"` from a specific `[section]` table within a TOML
/// file. Stops scanning at the next table header so a later table's `version`
/// (e.g. on a `[[commands]]` entry) doesn't get picked up by accident.
fn extract_versioned_toml_section(content: &str, section: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut in_section = false;
    let version_re = Regex::new(r#"^\s*version\s*=\s*"([^"]+)"\s*$"#).unwrap();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
            continue;
        }
        if in_section {
            if let Some(caps) = version_re.captures(line) {
                return Some(caps[1].to_string());
            }
        }
    }
    None
}

/// Replace the `version = "..."` line scoped to a specific `[section]` table.
/// Returns `Some(new_content)` if a replacement was made, `None` if either the
/// section or its `version` line was absent (so the caller knows whether to
/// touch the file). Preserves the original line-ending (LF / CRLF).
fn replace_versioned_toml_section(
    content: &str,
    section: &str,
    new_version: &str,
) -> Option<String> {
    let header = format!("[{section}]");
    let version_re = Regex::new(r#"^(\s*version\s*=\s*")[^"]+("\s*)$"#).unwrap();
    let crlf = content.contains("\r\n");
    let line_sep = if crlf { "\r\n" } else { "\n" };
    let trailing_newline = content.ends_with('\n');

    let mut in_section = false;
    let mut replaced = false;
    let mut out_lines: Vec<String> = Vec::new();

    for raw in content.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
            out_lines.push(line.to_string());
            continue;
        }
        if in_section && !replaced {
            if let Some(caps) = version_re.captures(line) {
                out_lines.push(format!("{}{}{}", &caps[1], new_version, &caps[2]));
                replaced = true;
                continue;
            }
        }
        out_lines.push(line.to_string());
    }

    if !replaced {
        return None;
    }

    let mut joined = out_lines.join(line_sep);
    if trailing_newline {
        joined.push_str(line_sep);
    }
    Some(joined)
}

fn apply_bump(current: &Version, bump: &str) -> Result<Version> {
    match bump {
        "major" => Ok(Version {
            major: current.major + 1,
            minor: 0,
            patch: 0,
        }),
        "minor" => Ok(Version {
            major: current.major,
            minor: current.minor + 1,
            patch: 0,
        }),
        "patch" => Ok(Version {
            major: current.major,
            minor: current.minor,
            patch: current.patch + 1,
        }),
        other => bail!(
            "Unknown bump level '{}'. Expected major, minor, or patch",
            other
        ),
    }
}

fn detect_version_files(dir: &Path) -> Vec<String> {
    let candidates: &[(&str, &str)] = &[
        ("plugin.toml", "fledge-plugin"),
        ("Cargo.toml", "rust"),
        ("package.json", "node"),
        ("pyproject.toml", "python"),
        ("setup.cfg", "python"),
        ("pom.xml", "java-maven"),
        ("build.gradle", "java-gradle"),
        ("build.gradle.kts", "java-gradle"),
    ];

    candidates
        .iter()
        .filter(|(name, _)| dir.join(name).exists())
        .map(|(name, _)| name.to_string())
        .collect()
}

fn bump_version_files(dir: &Path, new_version: &Version) -> Result<BumpResult> {
    let old = detect_current_version(dir).unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
    });
    let new_str = new_version.to_string();
    let mut bumped = Vec::new();

    // plugin.toml: only touch the version field inside the [plugin] table.
    // Other tables (e.g. `[[commands]]`) may have their own `version` and we
    // must not rewrite those.
    if let Ok(content) = std::fs::read_to_string(dir.join("plugin.toml")) {
        if let Some(updated) = replace_versioned_toml_section(&content, "plugin", &new_str) {
            std::fs::write(dir.join("plugin.toml"), updated.as_bytes())?;
            bumped.push("plugin.toml".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("Cargo.toml")) {
        let re = Regex::new(r#"(?m)^(version\s*=\s*")[^"]+(")"#).unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}${{2}}"));
            std::fs::write(dir.join("Cargo.toml"), updated.as_bytes())?;
            bumped.push("Cargo.toml".to_string());

            if dir.join("Cargo.lock").exists() {
                let lock = std::fs::read_to_string(dir.join("Cargo.lock"))?;
                if lock.contains(&format!("version = \"{}\"", old)) {
                    let status = Command::new("cargo")
                        .args(["generate-lockfile"])
                        .current_dir(dir)
                        .status()
                        .with_context(|| "running cargo generate-lockfile")?;
                    if status.success() {
                        bumped.push("Cargo.lock".to_string());
                    } else {
                        eprintln!("Warning: cargo generate-lockfile failed, Cargo.lock not staged");
                    }
                }
            }
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
        let re = Regex::new(r#"("version"\s*:\s*")[^"]+(")"#).unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}${{2}}"));
            std::fs::write(dir.join("package.json"), updated.as_bytes())?;
            bumped.push("package.json".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("pyproject.toml")) {
        let re = Regex::new(r#"(?m)^(version\s*=\s*")[^"]+(")"#).unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}${{2}}"));
            std::fs::write(dir.join("pyproject.toml"), updated.as_bytes())?;
            bumped.push("pyproject.toml".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("setup.cfg")) {
        let re = Regex::new(r"(?m)^(version\s*=\s*)\S+").unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}"));
            std::fs::write(dir.join("setup.cfg"), updated.as_bytes())?;
            bumped.push("setup.cfg".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("pom.xml")) {
        let re = Regex::new(r"(<version>)([^<]+)(</version>)").unwrap();
        let old_version_str = old.to_string();
        if let Some(caps) = re.captures(&content) {
            if caps
                .get(2)
                .map(|m| m.as_str().trim() == old_version_str.as_str())
                .unwrap_or(false)
            {
                let updated = re.replace(&content, format!("${{1}}{new_str}${{3}}"));
                std::fs::write(dir.join("pom.xml"), updated.as_bytes())?;
                bumped.push("pom.xml".to_string());
            }
        }
    }

    for name in &["build.gradle", "build.gradle.kts"] {
        let path = dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let re = Regex::new(r#"(version\s*=\s*)(["'])[^"']+["']"#).unwrap();
            if re.is_match(&content) {
                let updated = re.replace(&content, format!("${{1}}${{2}}{new_str}${{2}}"));
                std::fs::write(&path, updated.as_bytes())?;
                bumped.push(name.to_string());
            }
        }
    }

    // Also check for [release] config in fledge.toml for custom version files
    if let Ok(content) = std::fs::read_to_string(dir.join("fledge.toml")) {
        if let Ok(parsed) = content.parse::<toml::Value>() {
            if let Some(files) = parsed
                .get("release")
                .and_then(|r| r.get("files"))
                .and_then(|f| f.as_array())
            {
                for file_val in files {
                    if let Some(file_name) = file_val.as_str() {
                        if !bumped.iter().any(|b| b == file_name) {
                            let path = dir.join(file_name);
                            if path.exists() {
                                let canonical_path = path
                                    .canonicalize()
                                    .with_context(|| format!("canonicalizing '{}'", file_name))?;
                                let canonical_dir = dir
                                    .canonicalize()
                                    .with_context(|| "canonicalizing project dir")?;
                                if !canonical_path.starts_with(&canonical_dir) {
                                    bail!("Release file '{}' escapes project directory", file_name);
                                }
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let re = Regex::new(
                                        r#"(?m)(version\s*[=:]\s*["']?)(\d+\.\d+\.\d+)"#,
                                    )
                                    .unwrap();
                                    if re.is_match(&content) {
                                        let updated =
                                            re.replace(&content, format!("${{1}}{new_str}"));
                                        std::fs::write(&path, updated.as_bytes())?;
                                        bumped.push(file_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(BumpResult {
        old,
        new: new_version.clone(),
        files_bumped: bumped,
    })
}

fn generate_changelog_entry(dir: &Path, version: &Version, quiet: bool) -> Result<bool> {
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

fn classify_for_changelog(msg: &str) -> &'static str {
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
    ];

    for (prefix, label) in &prefixes {
        if msg.starts_with(prefix) && msg[prefix.len()..].starts_with([':', '(']) {
            return label;
        }
    }

    "Other"
}

fn strip_conventional_prefix(msg: &str) -> &str {
    if let Some(colon_pos) = msg.find(':') {
        let prefix = &msg[..colon_pos];
        let after = msg[colon_pos + 1..].trim_start();
        let base = if let Some(paren) = prefix.find('(') {
            &prefix[..paren]
        } else {
            prefix
        };
        let known = [
            "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
        ];
        if known.contains(&base) {
            return after;
        }
    }
    msg
}

fn create_release_commit(
    dir: &Path,
    version: &Version,
    bumped_files: &[String],
    has_changelog: bool,
    quiet: bool,
) -> Result<()> {
    let mut files_to_add: Vec<String> = bumped_files.to_vec();
    if has_changelog && dir.join("CHANGELOG.md").exists() {
        files_to_add.push("CHANGELOG.md".to_string());
    }

    if !files_to_add.is_empty() {
        let mut cmd = Command::new("git");
        cmd.arg("add").current_dir(dir);
        for f in &files_to_add {
            cmd.arg(f);
        }
        let output = cmd.output().context("staging release files")?;
        if !output.status.success() {
            bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    let msg = format!("chore: release v{version}");
    let mut commit_args = vec!["commit", "-m", &msg];
    if files_to_add.is_empty() {
        commit_args.push("--allow-empty");
    }
    let output = Command::new("git")
        .args(&commit_args)
        .current_dir(dir)
        .output()
        .context("creating release commit")?;

    if !output.status.success() {
        bail!(
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if !quiet {
        println!("  Created commit: {}", style(&msg).dim());
    }
    Ok(())
}

fn create_tag(dir: &Path, version: &Version, quiet: bool) -> Result<()> {
    let tag = format!("v{version}");

    let check = Command::new("git")
        .args(["tag", "-l", &tag])
        .current_dir(dir)
        .output()
        .context("checking existing tags")?;
    if !String::from_utf8_lossy(&check.stdout).trim().is_empty() {
        bail!(
            "Tag '{}' already exists. Delete it first with: git tag -d {}",
            tag,
            tag
        );
    }

    let output = Command::new("git")
        .args(["tag", "-a", &tag, "-m", &format!("Release {tag}")])
        .current_dir(dir)
        .output()
        .context("creating git tag")?;

    if !output.status.success() {
        bail!(
            "git tag failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if !quiet {
        println!("  Created tag: {}", style(&tag).cyan());
    }
    Ok(())
}

fn push_release(dir: &Path, version: &Version, has_tag: bool, quiet: bool) -> Result<()> {
    let output = Command::new("git")
        .args(["push"])
        .current_dir(dir)
        .output()
        .context("pushing release commit")?;

    if !output.status.success() {
        bail!(
            "git push failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if has_tag {
        let tag = format!("v{version}");
        let output = Command::new("git")
            .args(["push", "origin", &tag])
            .current_dir(dir)
            .output()
            .context("pushing release tag")?;

        if !output.status.success() {
            bail!(
                "git push tag failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    if !quiet {
        println!("  Pushed to remote");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn with_cwd<F: FnOnce() -> R, R>(dir: &Path, f: F) -> R {
        let _guard = CWD_LOCK.lock().unwrap();
        let saved = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        let result = f();
        let _ = std::env::set_current_dir(saved);
        result
    }

    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    fn commit_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
        Command::new("git")
            .args(["add", name])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("add {name}")])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn apply_bump_major() {
        let v = parse_version("1.2.3").unwrap();
        let bumped = apply_bump(&v, "major").unwrap();
        assert_eq!(bumped.to_string(), "2.0.0");
    }

    #[test]
    fn apply_bump_minor() {
        let v = parse_version("1.2.3").unwrap();
        let bumped = apply_bump(&v, "minor").unwrap();
        assert_eq!(bumped.to_string(), "1.3.0");
    }

    #[test]
    fn apply_bump_patch() {
        let v = parse_version("1.2.3").unwrap();
        let bumped = apply_bump(&v, "patch").unwrap();
        assert_eq!(bumped.to_string(), "1.2.4");
    }

    #[test]
    fn apply_bump_from_zero() {
        let v = parse_version("0.0.0").unwrap();
        assert_eq!(apply_bump(&v, "major").unwrap().to_string(), "1.0.0");
        assert_eq!(apply_bump(&v, "minor").unwrap().to_string(), "0.1.0");
        assert_eq!(apply_bump(&v, "patch").unwrap().to_string(), "0.0.1");
    }

    #[test]
    fn apply_bump_invalid_level() {
        let v = parse_version("1.2.3").unwrap();
        assert!(apply_bump(&v, "mega").is_err());
    }

    #[test]
    fn extract_toml_version_basic() {
        let content = r#"
[package]
name = "my-app"
version = "0.5.0"
edition = "2021"
"#;
        assert_eq!(extract_toml_version(content), Some("0.5.0".to_string()));
    }

    #[test]
    fn extract_toml_version_not_found() {
        assert_eq!(extract_toml_version("name = \"foo\""), None);
    }

    #[test]
    fn detect_version_files_rust() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"",
        )
        .unwrap();
        let files = detect_version_files(tmp.path());
        assert_eq!(files, vec!["Cargo.toml"]);
    }

    #[test]
    fn detect_version_files_node() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"version": "1.0.0"}"#).unwrap();
        let files = detect_version_files(tmp.path());
        assert_eq!(files, vec!["package.json"]);
    }

    #[test]
    fn detect_version_files_empty() {
        let tmp = TempDir::new().unwrap();
        let files = detect_version_files(tmp.path());
        assert!(files.is_empty());
    }

    #[test]
    fn detect_version_files_multiple() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "version = \"0.1.0\"").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let files = detect_version_files(tmp.path());
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn classify_conventional_commits() {
        assert_eq!(classify_for_changelog("feat: add release"), "Features");
        assert_eq!(classify_for_changelog("fix: handle null"), "Fixes");
        assert_eq!(
            classify_for_changelog("docs: update readme"),
            "Documentation"
        );
        assert_eq!(classify_for_changelog("chore: bump deps"), "Chores");
        assert_eq!(classify_for_changelog("feat(cli): add flag"), "Features");
        assert_eq!(classify_for_changelog("random message"), "Other");
    }

    #[test]
    fn strip_prefix_simple() {
        assert_eq!(
            strip_conventional_prefix("feat: add release"),
            "add release"
        );
        assert_eq!(
            strip_conventional_prefix("fix(core): null check"),
            "null check"
        );
        assert_eq!(strip_conventional_prefix("update readme"), "update readme");
    }

    #[test]
    fn strip_prefix_no_space_after_colon() {
        assert_eq!(strip_conventional_prefix("feat:add release"), "add release");
        assert_eq!(
            strip_conventional_prefix("fix(core):null check"),
            "null check"
        );
    }

    #[test]
    fn read_cargo_version_test() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"3.2.1\"\n",
        )
        .unwrap();
        assert_eq!(read_cargo_version(tmp.path()).unwrap(), "3.2.1");
    }

    #[test]
    fn read_package_json_version_test() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "test", "version": "2.0.0"}"#,
        )
        .unwrap();
        assert_eq!(read_package_json_version(tmp.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn read_python_version_test() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"test\"\nversion = \"1.5.0\"\n",
        )
        .unwrap();
        assert_eq!(read_python_version(tmp.path()).unwrap(), "1.5.0");
    }

    #[test]
    fn bump_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );
        let new_ver = parse_version("0.2.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"Cargo.toml".to_string()));
        let content = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"0.2.0\""));
    }

    #[test]
    fn bump_package_json() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "package.json",
            r#"{"name": "test", "version": "1.0.0"}"#,
        );
        let new_ver = parse_version("1.1.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"package.json".to_string()));
        let content = fs::read_to_string(tmp.path().join("package.json")).unwrap();
        assert!(content.contains("\"1.1.0\""));
    }

    #[test]
    fn bump_pyproject_toml() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"test\"\nversion = \"0.3.0\"\n",
        );
        let new_ver = parse_version("0.4.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"pyproject.toml".to_string()));
    }

    #[test]
    fn detect_version_from_plugin_toml() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "plugin.toml",
            "[plugin]\nname = \"fledge-deploy\"\nversion = \"0.3.0\"\n\n[[commands]]\nname = \"deploy\"\nbinary = \"bin/deploy\"\n",
        );
        let v = detect_current_version(tmp.path()).unwrap();
        assert_eq!(v.to_string(), "0.3.0");
    }

    #[test]
    fn bump_plugin_toml_only_touches_plugin_section() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        // Manifest has version inside [plugin] AND a `version` key elsewhere
        // (a `[[commands]]` table) — the bumper must only rewrite the [plugin]
        // one.
        let manifest = "[plugin]\nname = \"fledge-deploy\"\nversion = \"0.1.0\"\n\n[[commands]]\nname = \"deploy\"\nbinary = \"bin/deploy\"\nversion = \"99.99.99\"\n";
        commit_file(tmp.path(), "plugin.toml", manifest);
        let new_ver = parse_version("0.2.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"plugin.toml".to_string()));
        let updated = fs::read_to_string(tmp.path().join("plugin.toml")).unwrap();
        assert!(updated.contains("[plugin]\nname = \"fledge-deploy\"\nversion = \"0.2.0\""));
        // The bogus `version` on the command row stays put.
        assert!(updated.contains("version = \"99.99.99\""));
    }

    #[test]
    fn bump_plugin_toml_with_cargo_toml_keeps_them_in_sync() {
        // Rust plugins (e.g. fledge-plugin-metrics) carry both manifests and
        // expect both to bump together.
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "plugin.toml",
            "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n",
        );
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        );
        let new_ver = parse_version("0.2.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"plugin.toml".to_string()));
        assert!(result.files_bumped.contains(&"Cargo.toml".to_string()));
        let plugin = fs::read_to_string(tmp.path().join("plugin.toml")).unwrap();
        let cargo = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        assert!(plugin.contains("version = \"0.2.0\""));
        assert!(cargo.contains("version = \"0.2.0\""));
    }

    #[test]
    fn extract_versioned_section_skips_other_tables() {
        let toml = "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n\n[[commands]]\nname = \"y\"\nversion = \"99.0.0\"\n";
        assert_eq!(
            extract_versioned_toml_section(toml, "plugin"),
            Some("0.1.0".to_string())
        );
    }

    #[test]
    fn extract_versioned_section_returns_none_when_section_absent() {
        let toml = "[plugin]\nname = \"x\"\n";
        assert_eq!(extract_versioned_toml_section(toml, "plugin"), None);
    }

    #[test]
    fn replace_versioned_section_returns_none_when_no_match() {
        let toml = "[other]\nversion = \"1.0.0\"\n";
        assert_eq!(
            replace_versioned_toml_section(toml, "plugin", "2.0.0"),
            None
        );
    }

    #[test]
    fn replace_versioned_section_preserves_trailing_newline() {
        let toml = "[plugin]\nversion = \"0.1.0\"\n";
        let out = replace_versioned_toml_section(toml, "plugin", "0.2.0").unwrap();
        assert_eq!(out, "[plugin]\nversion = \"0.2.0\"\n");
    }

    #[test]
    fn bump_release_files_flake_nix() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );
        commit_file(
            tmp.path(),
            "flake.nix",
            "{ pname = \"x\"; version = \"0.1.0\"; }\n",
        );
        commit_file(
            tmp.path(),
            "fledge.toml",
            "[release]\nfiles = [\"flake.nix\"]\n",
        );
        let new_ver = parse_version("0.2.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"flake.nix".to_string()));
        let content = fs::read_to_string(tmp.path().join("flake.nix")).unwrap();
        assert!(content.contains("version = \"0.2.0\""));
    }

    #[test]
    fn preflight_checks_not_git() {
        let tmp = TempDir::new().unwrap();
        assert!(preflight_checks(tmp.path(), false).is_err());
    }

    #[test]
    fn preflight_checks_clean() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "test.txt", "hello");
        assert!(preflight_checks(tmp.path(), false).is_ok());
    }

    #[test]
    fn preflight_checks_dirty_allowed() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "test.txt", "hello");
        fs::write(tmp.path().join("dirty.txt"), "untracked").unwrap();
        assert!(preflight_checks(tmp.path(), true).is_ok());
    }

    #[test]
    fn preflight_checks_dirty_blocked() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "test.txt", "hello");
        fs::write(tmp.path().join("dirty.txt"), "untracked").unwrap();
        assert!(preflight_checks(tmp.path(), false).is_err());
    }

    #[test]
    fn resolve_explicit_version() {
        let tmp = TempDir::new().unwrap();
        let v = resolve_target_version(tmp.path(), "2.0.0").unwrap();
        assert_eq!(v.to_string(), "2.0.0");
    }

    #[test]
    fn resolve_bump_from_cargo() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"1.0.0\"\n",
        );
        let v = resolve_target_version(tmp.path(), "minor").unwrap();
        assert_eq!(v.to_string(), "1.1.0");
    }

    #[test]
    fn dry_run_no_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );

        let tmp_path = tmp.path().to_path_buf();
        let result = with_cwd(&tmp_path, || {
            run(ReleaseOptions {
                bump: "patch".to_string(),
                dry_run: true,
                no_tag: false,
                no_changelog: false,
                no_bump: false,
                push: false,
                pre_lane: None,
                allow_dirty: false,
                json: false,
            })
        });

        assert!(result.is_ok());
        let content = fs::read_to_string(tmp_path.join("Cargo.toml")).unwrap();
        assert!(content.contains("0.1.0"), "dry run should not modify files");
    }

    #[test]
    fn full_release_flow() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );

        Command::new("git")
            .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        commit_file(tmp.path(), "src.rs", "fn main() {}");

        let tmp_path = tmp.path().to_path_buf();
        let result = with_cwd(&tmp_path, || {
            run(ReleaseOptions {
                bump: "minor".to_string(),
                dry_run: false,
                no_tag: false,
                no_changelog: false,
                no_bump: false,
                push: false,
                pre_lane: None,
                allow_dirty: false,
                json: false,
            })
        });

        assert!(result.is_ok());

        let content = fs::read_to_string(tmp_path.join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"0.2.0\""));

        assert!(tmp_path.join("CHANGELOG.md").exists());

        let tag_output = Command::new("git")
            .args(["tag", "-l", "v0.2.0"])
            .current_dir(&tmp_path)
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&tag_output.stdout).contains("v0.2.0"));
    }

    #[test]
    fn release_tag_only_project() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "main.go", "package main");

        Command::new("git")
            .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        commit_file(tmp.path(), "main_test.go", "package main");

        let tmp_path = tmp.path().to_path_buf();
        let result = with_cwd(&tmp_path, || {
            run(ReleaseOptions {
                bump: "patch".to_string(),
                dry_run: false,
                no_tag: false,
                no_changelog: false,
                no_bump: false,
                push: false,
                pre_lane: None,
                allow_dirty: false,
                json: false,
            })
        });

        assert!(result.is_ok());

        let tag_output = Command::new("git")
            .args(["tag", "-l", "v0.1.1"])
            .current_dir(&tmp_path)
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&tag_output.stdout).contains("v0.1.1"));
    }

    #[test]
    fn changelog_entry_format() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "a.txt", "a");

        Command::new("git")
            .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        fs::write(tmp.path().join("b.txt"), "b").unwrap();
        Command::new("git")
            .args(["add", "b.txt"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feat: add feature b"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let tmp_path = tmp.path().to_path_buf();
        let version = parse_version("0.2.0").unwrap();
        let result = with_cwd(&tmp_path, || {
            generate_changelog_entry(&tmp_path, &version, false)
        });

        assert!(result.is_ok());
        let changelog = fs::read_to_string(tmp_path.join("CHANGELOG.md")).unwrap();
        assert!(changelog.contains("[v0.2.0]"));
        assert!(changelog.contains("### Features"));
        assert!(changelog.contains("add feature b"));
    }

    #[test]
    fn changelog_appends_to_existing() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());

        let existing = "# Changelog\n\n## [v0.1.0] - 2024-01-01\n\n### Features\n\n- initial\n";
        commit_file(tmp.path(), "CHANGELOG.md", existing);

        Command::new("git")
            .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        fs::write(tmp.path().join("new.txt"), "new").unwrap();
        Command::new("git")
            .args(["add", "new.txt"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "fix: patch bug"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let tmp_path = tmp.path().to_path_buf();
        let version = parse_version("0.1.1").unwrap();
        with_cwd(&tmp_path, || {
            generate_changelog_entry(&tmp_path, &version, false).unwrap();
        });

        let changelog = fs::read_to_string(tmp_path.join("CHANGELOG.md")).unwrap();
        assert!(changelog.contains("[v0.1.1]"));
        assert!(changelog.contains("[v0.1.0]"));
        let pos_new = changelog.find("[v0.1.1]").unwrap();
        let pos_old = changelog.find("[v0.1.0]").unwrap();
        assert!(
            pos_new < pos_old,
            "new entry should appear before old entry"
        );
    }

    #[test]
    fn read_maven_version_test() {
        let tmp = TempDir::new().unwrap();
        let pom = r#"<?xml version="1.0"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
    <version>1.3.0</version>
</project>"#;
        fs::write(tmp.path().join("pom.xml"), pom).unwrap();
        assert_eq!(read_maven_version(tmp.path()).unwrap(), "1.3.0");
    }

    #[test]
    fn read_gradle_version_test() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("build.gradle.kts"),
            "plugins { id(\"java\") }\nversion = \"2.1.0\"\n",
        )
        .unwrap();
        assert_eq!(read_gradle_version(tmp.path()).unwrap(), "2.1.0");
    }

    #[test]
    fn custom_release_files() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());

        let fledge_toml = r#"
[release]
files = ["version.txt"]
"#;
        commit_file(tmp.path(), "fledge.toml", fledge_toml);
        commit_file(tmp.path(), "version.txt", "version = \"0.1.0\"");
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );

        let new_ver = parse_version("0.2.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"version.txt".to_string()));

        let content = fs::read_to_string(tmp.path().join("version.txt")).unwrap();
        assert!(content.contains("0.2.0"));
    }

    #[test]
    fn read_gemspec_version_test() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("my_gem.gemspec"),
            r#"
Gem::Specification.new do |s|
  s.name = "my_gem"
  s.version = "1.4.2"
  s.summary = "A test gem"
end
"#,
        )
        .unwrap();
        assert_eq!(read_gemspec_version(tmp.path()).unwrap(), "1.4.2");
    }

    #[test]
    fn read_gemspec_version_single_quotes() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("my_gem.gemspec"),
            "Gem::Specification.new do |s|\n  s.version = '2.0.1'\nend\n",
        )
        .unwrap();
        assert_eq!(read_gemspec_version(tmp.path()).unwrap(), "2.0.1");
    }

    #[test]
    fn read_python_version_from_setup_cfg() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("setup.cfg"),
            "[metadata]\nname = my_pkg\nversion = 3.1.0\n",
        )
        .unwrap();
        assert_eq!(read_python_version(tmp.path()).unwrap(), "3.1.0");
    }

    #[test]
    fn read_python_version_pyproject_takes_priority() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"test\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("setup.cfg"),
            "[metadata]\nversion = 2.0.0\n",
        )
        .unwrap();
        assert_eq!(read_python_version(tmp.path()).unwrap(), "1.0.0");
    }

    #[test]
    fn duplicate_tag_prevented() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "test.txt", "hello");

        Command::new("git")
            .args(["tag", "-a", "v1.0.0", "-m", "v1.0.0"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let version = parse_version("1.0.0").unwrap();
        let result = create_tag(tmp.path(), &version, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("already exists"),
            "expected 'already exists' error, got: {err}"
        );
    }

    #[test]
    fn bump_setup_cfg() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "setup.cfg",
            "[metadata]\nname = test\nversion = 0.5.0\n",
        );
        commit_file(tmp.path(), "pyproject.toml", "[build-system]\n");

        let new_ver = parse_version("0.6.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"setup.cfg".to_string()));
        let content = fs::read_to_string(tmp.path().join("setup.cfg")).unwrap();
        assert!(content.contains("0.6.0"));
    }

    #[test]
    fn bump_pom_xml() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        let pom = r#"<?xml version="1.0"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
    <version>1.0.0</version>
</project>"#;
        commit_file(tmp.path(), "pom.xml", pom);

        let new_ver = parse_version("1.1.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result.files_bumped.contains(&"pom.xml".to_string()));
        let content = fs::read_to_string(tmp.path().join("pom.xml")).unwrap();
        assert!(content.contains("<version>1.1.0</version>"));
    }

    #[test]
    fn bump_gradle() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "build.gradle.kts",
            "plugins { id(\"java\") }\nversion = \"0.3.0\"\n",
        );

        let new_ver = parse_version("0.4.0").unwrap();
        let result = bump_version_files(tmp.path(), &new_ver).unwrap();
        assert!(result
            .files_bumped
            .contains(&"build.gradle.kts".to_string()));
        let content = fs::read_to_string(tmp.path().join("build.gradle.kts")).unwrap();
        assert!(content.contains("\"0.4.0\""));
    }

    #[test]
    fn changelog_created_fresh() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(tmp.path(), "a.txt", "a");

        let tmp_path = tmp.path().to_path_buf();
        let version = parse_version("0.1.0").unwrap();
        with_cwd(&tmp_path, || {
            generate_changelog_entry(&tmp_path, &version, false).unwrap();
        });

        let changelog = fs::read_to_string(tmp_path.join("CHANGELOG.md")).unwrap();
        assert!(changelog.starts_with("# Changelog"));
        assert!(changelog.contains("[v0.1.0]"));
    }

    #[test]
    fn release_with_no_tag_flag() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        commit_file(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );

        Command::new("git")
            .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        commit_file(tmp.path(), "src.rs", "fn main() {}");

        let tmp_path = tmp.path().to_path_buf();
        let result = with_cwd(&tmp_path, || {
            run(ReleaseOptions {
                bump: "patch".to_string(),
                dry_run: false,
                no_tag: true,
                no_changelog: true,
                no_bump: false,
                push: false,
                pre_lane: None,
                allow_dirty: false,
                json: false,
            })
        });

        assert!(result.is_ok());

        let content = fs::read_to_string(tmp_path.join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"0.1.1\""));

        let tag_output = Command::new("git")
            .args(["tag", "-l", "v0.1.1"])
            .current_dir(&tmp_path)
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&tag_output.stdout)
                .trim()
                .is_empty(),
            "no tag should be created with --no-tag"
        );
    }
}
