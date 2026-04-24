use anyhow::{bail, Context, Result};
use console::style;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const COMPANION_FILES: &[&str] = &["requirements.md", "tasks.md", "context.md", "testing.md"];

#[derive(Debug, Deserialize)]
struct SpecSyncConfig {
    specs_dir: Option<String>,
    #[serde(default)]
    required_sections: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpecFrontmatter {
    pub module: String,
    pub version: u32,
    pub status: String,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug)]
struct ValidationIssue {
    message: String,
    is_error: bool,
}

#[derive(Debug)]
struct SpecResult {
    name: String,
    version: u32,
    status: String,
    file_count: usize,
    section_count: usize,
    required_count: usize,
    issues: Vec<ValidationIssue>,
}

impl SpecResult {
    fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.is_error)
    }

    fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| !i.is_error)
    }

    fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.is_error).count()
    }

    fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| !i.is_error).count()
    }
}

pub fn run(action: SpecAction) -> Result<()> {
    let root = find_project_root();
    match action {
        SpecAction::Check { strict, json } => check(&root, strict, json),
        SpecAction::Init => init(&root),
        SpecAction::New { name } => new_spec(&root, &name),
        SpecAction::List { json } => list_specs(&root, json),
        SpecAction::Show { name, json } => show_spec(&root, &name, json),
    }
}

#[derive(Debug)]
pub enum SpecAction {
    Check { strict: bool, json: bool },
    Init,
    New { name: String },
    List { json: bool },
    Show { name: String, json: bool },
}

#[derive(Debug, Serialize)]
struct SpecSummary {
    name: String,
    version: u32,
    status: String,
    path: String,
    files: Vec<String>,
    section_count: usize,
    required_sections: usize,
    companions: Vec<String>,
    missing_companions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SpecDetail {
    name: String,
    version: u32,
    status: String,
    path: String,
    files: Vec<String>,
    sections: Vec<String>,
    companions: Vec<String>,
    missing_companions: Vec<String>,
}

/// A compact entry suitable for prompt-context indexes.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub name: String,
    pub version: u32,
    pub status: String,
    pub purpose: Option<String>,
    pub files: Vec<String>,
}

fn load_config(project_root: &Path) -> Result<SpecSyncConfig> {
    let config_path = project_root.join(".specsync/config.toml");
    if !config_path.exists() {
        bail!(
            "No .specsync/config.toml found. Run {} to initialize.",
            style("fledge spec init").cyan()
        );
    }
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: SpecSyncConfig =
        toml::from_str(&content).with_context(|| "Failed to parse .specsync/config.toml")?;
    Ok(config)
}

fn find_project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn parse_frontmatter(content: &str) -> Result<(SpecFrontmatter, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        bail!("No YAML frontmatter found (must start with ---)");
    }

    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("No closing --- for frontmatter"))?;

    let yaml_str = &after_first[..end];
    let body = &after_first[end + 4..];

    let fm = parse_yaml_frontmatter(yaml_str)?;
    Ok((fm, body.to_string()))
}

fn parse_yaml_frontmatter(yaml: &str) -> Result<SpecFrontmatter> {
    let mut module = None;
    let mut version = None;
    let mut status = None;
    let mut files = Vec::new();
    let mut current_list: Option<&str> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- ") {
            let value = rest.trim().to_string();
            if current_list == Some("files") {
                files.push(value);
            }
            continue;
        }

        current_list = None;

        if let Some((key, val)) = trimmed.split_once(':') {
            let key = key.trim();
            let val = val.trim();

            if val.is_empty() || val == "[]" {
                if key == "files" {
                    if val == "[]" {
                        files.clear();
                    } else {
                        current_list = Some("files");
                    }
                }
                continue;
            }

            match key {
                "module" => module = Some(val.to_string()),
                "version" => {
                    version = Some(
                        val.parse::<u32>()
                            .with_context(|| format!("Invalid version: {val}"))?,
                    );
                }
                "status" => status = Some(val.to_string()),
                "files" if val.starts_with('[') && val.ends_with(']') => {
                    let inner = &val[1..val.len() - 1];
                    files = inner
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    Ok(SpecFrontmatter {
        module: module.ok_or_else(|| anyhow::anyhow!("Missing required field: module"))?,
        version: version.ok_or_else(|| anyhow::anyhow!("Missing required field: version"))?,
        status: status.ok_or_else(|| anyhow::anyhow!("Missing required field: status"))?,
        files,
    })
}

fn extract_sections(body: &str) -> Vec<String> {
    let mut sections = Vec::new();
    for line in body.lines() {
        if let Some(section) = line.strip_prefix("## ") {
            sections.push(section.trim().to_string());
        }
    }
    sections
}

fn extract_purpose(body: &str) -> Option<String> {
    let mut in_purpose = false;
    let mut paragraph = String::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## ") {
            if in_purpose {
                break;
            }
            if trimmed == "## Purpose" {
                in_purpose = true;
            }
            continue;
        }
        if !in_purpose {
            continue;
        }
        if line.trim().is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(line.trim());
    }
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph)
    }
}

fn validate_spec(
    spec_path: &Path,
    project_root: &Path,
    required_sections: &[String],
) -> SpecResult {
    let content = match fs::read_to_string(spec_path) {
        Ok(c) => c,
        Err(e) => {
            return SpecResult {
                name: spec_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                version: 0,
                status: "unknown".to_string(),
                file_count: 0,
                section_count: 0,
                required_count: required_sections.len(),
                issues: vec![ValidationIssue {
                    message: format!("Failed to read: {e}"),
                    is_error: true,
                }],
            };
        }
    };

    let (fm, body) = match parse_frontmatter(&content) {
        Ok(r) => r,
        Err(e) => {
            return SpecResult {
                name: spec_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                version: 0,
                status: "unknown".to_string(),
                file_count: 0,
                section_count: 0,
                required_count: required_sections.len(),
                issues: vec![ValidationIssue {
                    message: format!("Invalid frontmatter: {e}"),
                    is_error: true,
                }],
            };
        }
    };

    let mut issues = Vec::new();

    let valid_statuses = [
        "draft",
        "review",
        "active",
        "stable",
        "deprecated",
        "archived",
    ];
    if !valid_statuses.contains(&fm.status.as_str()) {
        issues.push(ValidationIssue {
            message: format!(
                "Invalid status '{}' (expected one of: {valid_statuses:?})",
                fm.status
            ),
            is_error: true,
        });
    }

    for file in &fm.files {
        let file_path = project_root.join(file);
        if !file_path.exists() {
            issues.push(ValidationIssue {
                message: format!("file not found: {file}"),
                is_error: true,
            });
        }
    }

    let sections = extract_sections(&body);
    let mut missing_sections = Vec::new();
    for required in required_sections {
        if !sections.iter().any(|s| s == required) {
            missing_sections.push(required.clone());
        }
    }
    if !missing_sections.is_empty() {
        issues.push(ValidationIssue {
            message: format!("missing sections: {}", missing_sections.join(", ")),
            is_error: true,
        });
    }

    let spec_dir = spec_path.parent().unwrap_or(project_root);
    for companion in COMPANION_FILES {
        let companion_path = spec_dir.join(companion);
        if !companion_path.exists() {
            issues.push(ValidationIssue {
                message: format!("companion file missing: {companion}"),
                is_error: false,
            });
        }
    }

    SpecResult {
        name: fm.module.clone(),
        version: fm.version,
        status: fm.status.clone(),
        file_count: fm.files.len(),
        section_count: sections.len(),
        required_count: required_sections.len(),
        issues,
    }
}

fn check(root: &Path, strict: bool, json: bool) -> Result<()> {
    let config = load_config(root)?;
    let specs_dir = root.join(config.specs_dir.as_deref().unwrap_or("specs"));

    if !specs_dir.exists() {
        if json {
            let payload = serde_json::json!({
                "specs": [],
                "totals": { "checked": 0, "errors": 0, "warnings": 0 },
                "strict": strict,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "{} No specs directory found at {}",
                style("*").cyan().bold(),
                style(specs_dir.display()).dim()
            );
        }
        return Ok(());
    }

    let required_sections = if config.required_sections.is_empty() {
        vec![
            "Purpose".to_string(),
            "Public API".to_string(),
            "Invariants".to_string(),
            "Behavioral Examples".to_string(),
            "Error Cases".to_string(),
            "Dependencies".to_string(),
            "Change Log".to_string(),
        ]
    } else {
        config.required_sections.clone()
    };

    let mut results: Vec<SpecResult> = Vec::new();

    for entry in WalkDir::new(&specs_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".spec.md") {
                results.push(validate_spec(path, root, &required_sections));
            }
        }
    }

    if results.is_empty() {
        if json {
            let payload = serde_json::json!({
                "specs": [],
                "totals": { "checked": 0, "errors": 0, "warnings": 0 },
                "strict": strict,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "{} No spec files found in {}",
                style("*").cyan().bold(),
                style(specs_dir.display()).dim()
            );
        }
        return Ok(());
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));

    let mut total_errors = 0;
    let mut total_warnings = 0;
    for result in &results {
        total_errors += result.error_count();
        total_warnings += result.warning_count();
    }

    if json {
        let specs_payload: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let errors: Vec<&str> = r
                    .issues
                    .iter()
                    .filter(|i| i.is_error)
                    .map(|i| i.message.as_str())
                    .collect();
                let warnings: Vec<&str> = r
                    .issues
                    .iter()
                    .filter(|i| !i.is_error)
                    .map(|i| i.message.as_str())
                    .collect();
                serde_json::json!({
                    "name": r.name,
                    "version": r.version,
                    "status": r.status,
                    "file_count": r.file_count,
                    "section_count": r.section_count,
                    "required_count": r.required_count,
                    "errors": errors,
                    "warnings": warnings,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "specs": specs_payload,
            "totals": {
                "checked": results.len(),
                "errors": total_errors,
                "warnings": total_warnings,
            },
            "strict": strict,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        if total_errors > 0 || (strict && total_warnings > 0) {
            std::process::exit(1);
        }
        return Ok(());
    }

    for result in &results {
        if result.has_errors() || (strict && result.has_warnings()) {
            print!(
                "{} {} (v{}, {})",
                style("❌").red().bold(),
                style(&result.name).red(),
                result.version,
                result.status,
            );
        } else if result.has_warnings() {
            print!(
                "{} {} (v{}, {})",
                style("⚠️").yellow().bold(),
                style(&result.name).yellow(),
                result.version,
                result.status,
            );
        } else {
            print!(
                "{} {} (v{}, {})",
                style("✅").green().bold(),
                style(&result.name).green(),
                result.version,
                result.status,
            );
        }

        println!(
            " — {} {}, {}/{} sections",
            result.file_count,
            if result.file_count == 1 {
                "file"
            } else {
                "files"
            },
            result.section_count,
            result.required_count,
        );

        for issue in &result.issues {
            if issue.is_error {
                println!("    {} {}", style("error:").red(), issue.message);
            } else {
                println!("    {} {}", style("warn:").yellow(), issue.message);
            }
        }
    }

    println!();
    println!(
        "  {} specs checked, {} {}, {} {}",
        results.len(),
        total_errors,
        if total_errors == 1 { "error" } else { "errors" },
        total_warnings,
        if total_warnings == 1 {
            "warning"
        } else {
            "warnings"
        },
    );

    if total_errors > 0 || (strict && total_warnings > 0) {
        if strict && total_warnings > 0 && total_errors == 0 {
            println!(
                "  {}",
                style("(warnings treated as errors in strict mode)").dim()
            );
        }
        std::process::exit(1);
    }

    Ok(())
}

fn find_spec_files(specs_dir: &Path) -> Vec<PathBuf> {
    let mut spec_paths = Vec::new();
    for entry in WalkDir::new(specs_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".spec.md") {
                spec_paths.push(path.to_path_buf());
            }
        }
    }
    spec_paths
}

fn classify_companions(spec_dir: &Path) -> (Vec<String>, Vec<String>) {
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for companion in COMPANION_FILES {
        if spec_dir.join(companion).exists() {
            present.push((*companion).to_string());
        } else {
            missing.push((*companion).to_string());
        }
    }
    (present, missing)
}

fn specs_dir_from_config(root: &Path) -> Result<PathBuf> {
    let config = load_config(root)?;
    Ok(root.join(config.specs_dir.as_deref().unwrap_or("specs")))
}

fn validate_module_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Module name cannot be empty");
    }
    if name == "." || name == ".." {
        bail!("Invalid module name '{name}'");
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        bail!("Invalid module name '{name}': may not contain path separators or '..'");
    }
    Ok(())
}

/// Collect a compact, sorted index of every spec in the project.
/// Intended for feeding into LLM prompts or other machine-readable consumers.
pub fn collect_index(root: &Path) -> Result<Vec<IndexEntry>> {
    let specs_dir = specs_dir_from_config(root)?;
    if !specs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for path in find_spec_files(&specs_dir) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok((fm, body)) = parse_frontmatter(&content) else {
            continue;
        };
        entries.push(IndexEntry {
            name: fm.module,
            version: fm.version,
            status: fm.status,
            purpose: extract_purpose(&body),
            files: fm.files,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Render an index as a compact markdown block suitable for prompt injection.
pub fn render_index_markdown(entries: &[IndexEntry]) -> String {
    let mut out = String::from("## Available specs\n\n");
    out.push_str(
        "This project documents each module in `specs/<name>/`. \
         Run `fledge spec show <name>` for the full detail.\n\n",
    );
    for entry in entries {
        let purpose = entry
            .purpose
            .as_deref()
            .unwrap_or("(no purpose documented)");
        let files = if entry.files.is_empty() {
            String::new()
        } else {
            format!(" — {}", entry.files.join(", "))
        };
        out.push_str(&format!(
            "- **{}** v{} ({}){} — {}\n",
            entry.name, entry.version, entry.status, files, purpose,
        ));
    }
    out
}

/// Return every module name that has a `specs/<name>/<name>.spec.md` file.
pub fn all_module_names(root: &Path) -> Result<Vec<String>> {
    Ok(collect_index(root)?.into_iter().map(|e| e.name).collect())
}

/// Return the module names whose `files:` frontmatter matches any of the given
/// paths, or whose `<specs_dir>/<name>/` directory contains any of them.
///
/// Used by `fledge review` to automatically include the right spec context
/// when reviewing a diff. Silent on specs that fail to parse — a broken
/// spec should never block a review.
///
/// Honors the `specs_dir` key from `.specsync/config.toml`, so projects that
/// put specs under e.g. `docs/specs/` match correctly.
pub fn specs_for_changed_files(root: &Path, changed_files: &[String]) -> Result<Vec<String>> {
    let index = collect_index(root)?;
    // Best-effort: if config is unreadable for any reason, fall back to "specs".
    let specs_dir_name = load_config(root)
        .ok()
        .and_then(|c| c.specs_dir)
        .unwrap_or_else(|| "specs".to_string());
    let specs_dir_trimmed = specs_dir_name.trim_end_matches('/');

    let mut matched = Vec::new();
    for entry in &index {
        let files_match = entry
            .files
            .iter()
            .any(|f| changed_files.iter().any(|c| c == f));
        let spec_prefix = format!("{specs_dir_trimmed}/{}/", entry.name);
        let dir_match = changed_files.iter().any(|c| c.starts_with(&spec_prefix));
        if files_match || dir_match {
            matched.push(entry.name.clone());
        }
    }
    matched.sort();
    matched.dedup();
    Ok(matched)
}

/// Load the full spec bundle for a single module: its `.spec.md` plus whichever
/// companion files exist, concatenated as a single markdown block with headers.
pub fn load_module_bundle(root: &Path, name: &str) -> Result<String> {
    validate_module_name(name)?;
    let specs_dir = specs_dir_from_config(root)?;
    let module_dir = specs_dir.join(name);
    let spec_path = module_dir.join(format!("{name}.spec.md"));
    if !spec_path.exists() {
        bail!(
            "No spec found for '{}' (looked at {})",
            name,
            spec_path.display()
        );
    }

    let mut bundle = String::new();
    bundle.push_str(&format!("## Spec bundle: {name}\n\n"));

    let spec_content = fs::read_to_string(&spec_path)
        .with_context(|| format!("reading {}", spec_path.display()))?;
    bundle.push_str(&format!("### `{name}.spec.md`\n\n"));
    bundle.push_str(spec_content.trim_end());
    bundle.push_str("\n\n");

    for companion in COMPANION_FILES {
        let companion_path = module_dir.join(companion);
        if !companion_path.exists() {
            continue;
        }
        let companion_content = fs::read_to_string(&companion_path)
            .with_context(|| format!("reading {}", companion_path.display()))?;
        bundle.push_str(&format!("### `{companion}`\n\n"));
        bundle.push_str(companion_content.trim_end());
        bundle.push_str("\n\n");
    }

    Ok(bundle)
}

fn build_summary(spec_path: &Path, root: &Path, required_count: usize) -> Result<SpecSummary> {
    let content = fs::read_to_string(spec_path)
        .with_context(|| format!("reading {}", spec_path.display()))?;
    let (fm, body) = parse_frontmatter(&content)
        .with_context(|| format!("parsing frontmatter in {}", spec_path.display()))?;
    let sections = extract_sections(&body);
    let (companions, missing_companions) = classify_companions(spec_path.parent().unwrap_or(root));
    let rel_path = spec_path
        .strip_prefix(root)
        .unwrap_or(spec_path)
        .display()
        .to_string();
    Ok(SpecSummary {
        name: fm.module,
        version: fm.version,
        status: fm.status,
        path: rel_path,
        files: fm.files,
        section_count: sections.len(),
        required_sections: required_count,
        companions,
        missing_companions,
    })
}

fn list_specs(root: &Path, json: bool) -> Result<()> {
    let config = load_config(root)?;
    let specs_dir = root.join(config.specs_dir.as_deref().unwrap_or("specs"));
    let required_count = if config.required_sections.is_empty() {
        7
    } else {
        config.required_sections.len()
    };

    if !specs_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!(
                "{} No specs directory found at {}",
                style("*").cyan().bold(),
                style(specs_dir.display()).dim()
            );
        }
        return Ok(());
    }

    let mut summaries: Vec<SpecSummary> = Vec::new();
    let mut parse_errors: Vec<(PathBuf, String)> = Vec::new();
    for path in find_spec_files(&specs_dir) {
        match build_summary(&path, root, required_count) {
            Ok(summary) => summaries.push(summary),
            Err(e) => parse_errors.push((path, e.to_string())),
        }
    }
    summaries.sort_by(|a, b| a.name.cmp(&b.name));

    if json {
        println!("{}", serde_json::to_string_pretty(&summaries)?);
        return Ok(());
    }

    if summaries.is_empty() && parse_errors.is_empty() {
        println!(
            "{} No spec files found in {}",
            style("*").cyan().bold(),
            style(specs_dir.display()).dim()
        );
        return Ok(());
    }

    for summary in &summaries {
        let status_marker = match summary.status.as_str() {
            "active" | "stable" => style("●").green(),
            "draft" | "review" => style("●").yellow(),
            "deprecated" | "archived" => style("●").red(),
            _ => style("●").dim(),
        };
        println!(
            "{} {} {} ({})",
            status_marker,
            style(&summary.name).bold(),
            style(format!("v{}", summary.version)).dim(),
            summary.status,
        );
        println!(
            "    {} — {} source {}, {}/{} sections, {} companion {}",
            style(&summary.path).dim(),
            summary.files.len(),
            if summary.files.len() == 1 {
                "file"
            } else {
                "files"
            },
            summary.section_count,
            summary.required_sections,
            summary.companions.len(),
            if summary.companions.len() == 1 {
                "file"
            } else {
                "files"
            },
        );
        if !summary.missing_companions.is_empty() {
            println!(
                "    {} {}",
                style("missing:").yellow(),
                summary.missing_companions.join(", ")
            );
        }
    }

    for (path, err) in &parse_errors {
        println!(
            "{} {} — {}",
            style("❌").red().bold(),
            path.display(),
            style(err).red()
        );
    }

    println!();
    println!("  {} spec(s) found", summaries.len());
    Ok(())
}

fn show_spec(root: &Path, name: &str, json: bool) -> Result<()> {
    validate_module_name(name)?;
    let config = load_config(root)?;
    let specs_dir = root.join(config.specs_dir.as_deref().unwrap_or("specs"));
    let spec_path = specs_dir.join(name).join(format!("{name}.spec.md"));

    if !spec_path.exists() {
        bail!(
            "No spec found for '{}'. Looked at {}. Run {} to see available specs.",
            name,
            spec_path.display(),
            style("fledge spec list").cyan()
        );
    }

    let content = fs::read_to_string(&spec_path)
        .with_context(|| format!("reading {}", spec_path.display()))?;
    let (fm, body) = parse_frontmatter(&content)
        .with_context(|| format!("parsing frontmatter in {}", spec_path.display()))?;
    let sections = extract_sections(&body);
    let spec_dir = spec_path.parent().unwrap_or(root);
    let (companions, missing_companions) = classify_companions(spec_dir);
    let rel_path = spec_path
        .strip_prefix(root)
        .unwrap_or(&spec_path)
        .display()
        .to_string();

    let detail = SpecDetail {
        name: fm.module,
        version: fm.version,
        status: fm.status,
        path: rel_path,
        files: fm.files,
        sections,
        companions,
        missing_companions,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&detail)?);
        return Ok(());
    }

    println!(
        "{} {} ({})",
        style(&detail.name).bold().cyan(),
        style(format!("v{}", detail.version)).dim(),
        detail.status,
    );
    println!("  path: {}", style(&detail.path).dim());

    if detail.files.is_empty() {
        println!("  source files: {}", style("(none)").dim());
    } else {
        println!("  source files:");
        for file in &detail.files {
            println!("    - {file}");
        }
    }

    if detail.sections.is_empty() {
        println!("  sections: {}", style("(none)").dim());
    } else {
        println!("  sections ({}):", detail.sections.len());
        for section in &detail.sections {
            println!("    - {section}");
        }
    }

    if detail.companions.is_empty() {
        println!("  companions: {}", style("(none present)").yellow());
    } else {
        println!("  companions:");
        for companion in &detail.companions {
            println!("    {} {}", style("✓").green(), companion);
        }
    }
    for companion in &detail.missing_companions {
        println!("    {} {}", style("✗").yellow(), companion);
    }

    Ok(())
}

fn init(root: &Path) -> Result<()> {
    let specsync_dir = root.join(".specsync");

    if specsync_dir.exists() {
        bail!(".specsync/ already exists. Remove it first to re-initialize.");
    }

    let specs_dir = root.join("specs");

    fs::create_dir_all(&specsync_dir)?;

    let project_name = root
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let config_content = r#"# spec-sync v4 configuration
# Docs: https://github.com/CorvidLabs/spec-sync

specs_dir = "specs"
source_dirs = ["src"]
exclude_dirs = []
exclude_patterns = []
required_sections = ["Purpose", "Public API", "Invariants", "Behavioral Examples", "Error Cases", "Dependencies", "Change Log"]
enforcement = "strict"

[lifecycle]
track_history = false
"#;

    let registry_content = format!(
        r#"[registry]
name = "{project_name}"

[specs]
"#
    );

    let gitignore_content = r#"backup-3x/
config.local.toml
hashes.json
"#;

    fs::write(specsync_dir.join("config.toml"), config_content)?;
    println!(
        "{} Created .specsync/config.toml",
        style("✅").green().bold()
    );

    fs::write(specsync_dir.join("registry.toml"), registry_content)?;
    println!(
        "{} Created .specsync/registry.toml",
        style("✅").green().bold()
    );

    fs::write(specsync_dir.join(".gitignore"), gitignore_content)?;
    println!(
        "{} Created .specsync/.gitignore",
        style("✅").green().bold()
    );

    fs::write(specsync_dir.join("version"), "4.3.1\n")?;
    println!("{} Created .specsync/version", style("✅").green().bold());

    if !specs_dir.exists() {
        fs::create_dir_all(&specs_dir)?;
        println!("{} Created specs/", style("✅").green().bold());
    }

    println!();
    println!(
        "  Spec-sync initialized. Run {} to create your first spec.",
        style("fledge spec new <name>").cyan()
    );

    Ok(())
}

fn new_spec(root: &Path, name: &str) -> Result<()> {
    validate_module_name(name)?;
    let config = load_config(root)?;
    let specs_dir = root.join(config.specs_dir.as_deref().unwrap_or("specs"));
    let spec_dir = specs_dir.join(name);

    if spec_dir.exists() {
        bail!("Spec directory already exists: {}", spec_dir.display());
    }

    fs::create_dir_all(&spec_dir)?;

    let spec_content = format!(
        r#"---
module: {name}
version: 1
status: draft
files:
  - src/{name}.rs

db_tables: []
depends_on: []
---

# {title}

## Purpose

<!-- Describe what this module does and why it exists. -->

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| | |

### Structs & Enums

| Type | Description |
|------|-------------|
| | |

### Traits

| Trait | Description |
|-------|-------------|
| | |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| | | |

## Invariants

1. <!-- List invariants that must always hold. -->

## Behavioral Examples

```
Given ...
When ...
Then ...
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| | | |

## Dependencies

- None

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | {date} | Initial spec |
"#,
        name = name,
        title = to_title_case(name),
        date = chrono::Local::now().format("%Y-%m-%d"),
    );

    let requirements_content = format!(
        r#"---
spec: {name}.spec.md
---

## User Stories

- As a developer, I want to <!-- describe the goal -->

## Acceptance Criteria

- <!-- List measurable acceptance criteria. -->

## Constraints

- <!-- List any constraints or limitations. -->

## Out of Scope

- <!-- List anything explicitly excluded. -->
"#
    );

    let tasks_content = format!(
        r#"---
spec: {name}.spec.md
---

## Tasks

- [ ] Write spec
- [ ] Implement module
- [ ] Write tests
"#
    );

    let context_content = format!(
        r#"---
spec: {name}.spec.md
---

## Context

<!-- Describe the context and motivation for this module. -->

## Related Modules

- <!-- List related modules or specs. -->

## Design Decisions

- <!-- Document key design decisions and their rationale. -->
"#
    );

    let testing_content = format!(
        r#"---
spec: {name}.spec.md
---

## Test Plan

### Unit Tests

- <!-- List unit test scenarios. -->

### Integration Tests

- <!-- List integration test scenarios. -->
"#
    );

    fs::write(spec_dir.join(format!("{name}.spec.md")), &spec_content)?;
    println!(
        "{} Created specs/{name}/{name}.spec.md",
        style("✅").green().bold()
    );

    fs::write(spec_dir.join("requirements.md"), &requirements_content)?;
    println!(
        "{} Created specs/{name}/requirements.md",
        style("✅").green().bold()
    );

    fs::write(spec_dir.join("tasks.md"), &tasks_content)?;
    println!(
        "{} Created specs/{name}/tasks.md",
        style("✅").green().bold()
    );

    fs::write(spec_dir.join("context.md"), &context_content)?;
    println!(
        "{} Created specs/{name}/context.md",
        style("✅").green().bold()
    );

    fs::write(spec_dir.join("testing.md"), &testing_content)?;
    println!(
        "{} Created specs/{name}/testing.md",
        style("✅").green().bold()
    );

    // Update registry
    let registry_path = root.join(".specsync/registry.toml");
    if registry_path.exists() {
        let mut registry = fs::read_to_string(&registry_path)?;
        let entry = format!("{name} = \"specs/{name}/{name}.spec.md\"\n");
        if !registry.contains(&format!("{name} =")) {
            registry.push_str(&entry);
            fs::write(&registry_path, &registry)?;
        }
    }

    println!();
    println!(
        "  Spec module '{}' created. Edit {} to get started.",
        style(name).green(),
        style(format!("specs/{name}/{name}.spec.md")).cyan()
    );

    Ok(())
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
module: init
version: 4
status: active
files:
  - src/init.rs
  - src/main.rs

db_tables: []
depends_on:
  - templates
---

# Init

## Purpose

Test purpose.
"#;
        let (fm, body) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.module, "init");
        assert_eq!(fm.version, 4);
        assert_eq!(fm.status, "active");
        assert_eq!(fm.files, vec!["src/init.rs", "src/main.rs"]);
        assert!(body.contains("## Purpose"));
    }

    #[test]
    fn test_parse_frontmatter_missing_module() {
        let content = r#"---
version: 1
status: draft
files: []
---
body
"#;
        let err = parse_frontmatter(content).unwrap_err();
        assert!(err.to_string().contains("module"));
    }

    #[test]
    fn test_parse_frontmatter_missing_version() {
        let content = r#"---
module: test
status: draft
files: []
---
body
"#;
        let err = parse_frontmatter(content).unwrap_err();
        assert!(err.to_string().contains("version"));
    }

    #[test]
    fn test_parse_frontmatter_missing_status() {
        let content = r#"---
module: test
version: 1
files: []
---
body
"#;
        let err = parse_frontmatter(content).unwrap_err();
        assert!(err.to_string().contains("status"));
    }

    #[test]
    fn test_parse_frontmatter_no_delimiters() {
        let content = "no frontmatter here";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(err.to_string().contains("---"));
    }

    #[test]
    fn test_parse_frontmatter_no_closing() {
        let content = "---\nmodule: test\n";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(err.to_string().contains("closing"));
    }

    #[test]
    fn test_extract_sections() {
        let body = r#"
# Title

## Purpose

Some text.

## Public API

More text.

## Invariants

1. First
"#;
        let sections = extract_sections(body);
        assert_eq!(sections, vec!["Purpose", "Public API", "Invariants"]);
    }

    #[test]
    fn test_extract_sections_empty() {
        let body = "No sections here, just text.";
        let sections = extract_sections(body);
        assert!(sections.is_empty());
    }

    #[test]
    fn test_extract_purpose_happy_path() {
        let body = "\n## Purpose\n\nA short description.\n\n## Public API\n\ntext\n";
        assert_eq!(extract_purpose(body), Some("A short description.".into()));
    }

    #[test]
    fn test_extract_purpose_multiline_joined() {
        let body = "## Purpose\n\nLine one\nline two\n\n## Next\n";
        assert_eq!(extract_purpose(body), Some("Line one line two".into()));
    }

    #[test]
    fn test_extract_purpose_missing_section() {
        let body = "## Public API\n\ntext\n";
        assert_eq!(extract_purpose(body), None);
    }

    fn scaffold_min_project(tmp: &TempDir, modules: &[&str]) {
        let specsync = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync).unwrap();
        fs::write(
            specsync.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();
        for name in modules {
            let dir = tmp.path().join(format!("specs/{name}"));
            fs::create_dir_all(&dir).unwrap();
            let spec = format!(
                "---\nmodule: {name}\nversion: 1\nstatus: active\nfiles: []\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nPurpose of {name}.\n\n## Public API\n\n## Invariants\n\n## Behavioral Examples\n\n## Error Cases\n\n## Dependencies\n\n## Change Log\n"
            );
            fs::write(dir.join(format!("{name}.spec.md")), spec).unwrap();
            fs::write(dir.join("requirements.md"), "---\nspec: x\n---\nreq body\n").unwrap();
            fs::write(dir.join("context.md"), "---\nspec: x\n---\ncontext body\n").unwrap();
        }
    }

    #[test]
    fn test_collect_index_sorted_with_purpose() {
        let tmp = TempDir::new().unwrap();
        scaffold_min_project(&tmp, &["zebra", "alpha", "mango"]);

        let entries = collect_index(tmp.path()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mango", "zebra"]);
        assert_eq!(entries[0].purpose, Some("Purpose of alpha.".into()));
        assert_eq!(entries[0].version, 1);
        assert_eq!(entries[0].status, "active");
    }

    #[test]
    fn test_collect_index_empty_project() {
        let tmp = TempDir::new().unwrap();
        scaffold_min_project(&tmp, &[]);
        let entries = collect_index(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_render_index_markdown_contains_entries() {
        let entries = vec![
            IndexEntry {
                name: "foo".into(),
                version: 2,
                status: "active".into(),
                purpose: Some("Does foo.".into()),
                files: vec!["src/foo.rs".into()],
            },
            IndexEntry {
                name: "bar".into(),
                version: 1,
                status: "draft".into(),
                purpose: None,
                files: Vec::new(),
            },
        ];
        let md = render_index_markdown(&entries);
        assert!(md.contains("## Available specs"));
        assert!(md.contains("**foo** v2 (active)"));
        assert!(md.contains("Does foo."));
        assert!(md.contains("**bar** v1 (draft)"));
        assert!(md.contains("(no purpose documented)"));
    }

    #[test]
    fn test_all_module_names_sorted() {
        let tmp = TempDir::new().unwrap();
        scaffold_min_project(&tmp, &["beta", "alpha"]);
        let names = all_module_names(tmp.path()).unwrap();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_load_module_bundle_includes_spec_and_companions() {
        let tmp = TempDir::new().unwrap();
        scaffold_min_project(&tmp, &["alpha"]);
        let bundle = load_module_bundle(tmp.path(), "alpha").unwrap();
        assert!(bundle.contains("## Spec bundle: alpha"));
        assert!(bundle.contains("### `alpha.spec.md`"));
        assert!(bundle.contains("Purpose of alpha."));
        assert!(bundle.contains("### `requirements.md`"));
        assert!(bundle.contains("req body"));
        assert!(bundle.contains("### `context.md`"));
        assert!(bundle.contains("context body"));
        // tasks and testing not scaffolded, so not present
        assert!(!bundle.contains("### `tasks.md`"));
        assert!(!bundle.contains("### `testing.md`"));
    }

    #[test]
    fn test_load_module_bundle_missing_module_errors() {
        let tmp = TempDir::new().unwrap();
        scaffold_min_project(&tmp, &[]);
        let err = load_module_bundle(tmp.path(), "ghost").unwrap_err();
        assert!(err.to_string().contains("No spec found"));
    }

    #[test]
    fn test_load_module_bundle_rejects_path_traversal() {
        let tmp = TempDir::new().unwrap();
        scaffold_min_project(&tmp, &["real"]);

        for bad in ["../evil", "..\\evil", "foo/bar", "foo\\bar", "..", ".", ""] {
            let err = load_module_bundle(tmp.path(), bad).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("Invalid module name") || msg.contains("cannot be empty"),
                "expected rejection for '{bad}', got: {msg}"
            );
        }
    }

    #[test]
    fn test_validate_module_name_allows_normal_names() {
        assert!(validate_module_name("trust").is_ok());
        assert!(validate_module_name("create_template").is_ok());
        assert!(validate_module_name("plugin-protocol").is_ok());
    }

    fn scaffold_project_with_source_specs(tmp: &TempDir) {
        let specsync = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync).unwrap();
        fs::write(
            specsync.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();

        for (name, source_files) in [
            ("trust", vec!["src/trust.rs"]),
            ("ask", vec!["src/ask.rs"]),
            ("work", vec!["src/work.rs"]),
        ] {
            let dir = tmp.path().join(format!("specs/{name}"));
            fs::create_dir_all(&dir).unwrap();
            let files_yaml = source_files
                .iter()
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n");
            let spec = format!(
                "---\nmodule: {name}\nversion: 1\nstatus: active\nfiles:\n{files_yaml}\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n"
            );
            fs::write(dir.join(format!("{name}.spec.md")), spec).unwrap();
        }
    }

    #[test]
    fn test_specs_for_changed_files_matches_via_frontmatter_files() {
        let tmp = TempDir::new().unwrap();
        scaffold_project_with_source_specs(&tmp);

        let changed = vec!["src/trust.rs".to_string(), "src/ask.rs".to_string()];
        let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
        assert_eq!(matched, vec!["ask", "trust"]);
    }

    #[test]
    fn test_specs_for_changed_files_matches_via_spec_directory() {
        let tmp = TempDir::new().unwrap();
        scaffold_project_with_source_specs(&tmp);

        let changed = vec!["specs/trust/context.md".to_string()];
        let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
        assert_eq!(matched, vec!["trust"]);
    }

    #[test]
    fn test_specs_for_changed_files_deduplicates() {
        let tmp = TempDir::new().unwrap();
        scaffold_project_with_source_specs(&tmp);

        // Both trust.rs and specs/trust/context.md → single match
        let changed = vec![
            "src/trust.rs".to_string(),
            "specs/trust/context.md".to_string(),
        ];
        let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
        assert_eq!(matched, vec!["trust"]);
    }

    #[test]
    fn test_specs_for_changed_files_no_match() {
        let tmp = TempDir::new().unwrap();
        scaffold_project_with_source_specs(&tmp);

        let changed = vec!["README.md".to_string(), "Cargo.toml".to_string()];
        let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
        assert!(matched.is_empty());
    }

    #[test]
    fn test_specs_for_changed_files_empty_input() {
        let tmp = TempDir::new().unwrap();
        scaffold_project_with_source_specs(&tmp);
        let matched = specs_for_changed_files(tmp.path(), &[]).unwrap();
        assert!(matched.is_empty());
    }

    #[test]
    fn test_specs_for_changed_files_honors_custom_specs_dir() {
        let tmp = TempDir::new().unwrap();
        let specsync = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync).unwrap();
        fs::write(
            specsync.join("config.toml"),
            "specs_dir = \"docs/specs\"\nrequired_sections = []\n",
        )
        .unwrap();
        let dir = tmp.path().join("docs/specs/trust");
        fs::create_dir_all(&dir).unwrap();
        let spec = "---\nmodule: trust\nversion: 1\nstatus: active\nfiles:\n  - src/trust.rs\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n";
        fs::write(dir.join("trust.spec.md"), spec).unwrap();

        // Match via `docs/specs/trust/...` prefix, not `specs/trust/...`
        let changed = vec!["docs/specs/trust/context.md".to_string()];
        let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
        assert_eq!(matched, vec!["trust"]);

        // Changing a file under the legacy `specs/...` path should NOT match
        // when the project uses a custom specs_dir
        let changed_wrong = vec!["specs/trust/context.md".to_string()];
        let matched_wrong = specs_for_changed_files(tmp.path(), &changed_wrong).unwrap();
        assert!(matched_wrong.is_empty());
    }

    #[test]
    fn test_validate_spec_all_valid() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path().join("specs/mymod");
        fs::create_dir_all(&specs_dir).unwrap();

        let src_file = tmp.path().join("src/mymod.rs");
        fs::create_dir_all(src_file.parent().unwrap()).unwrap();
        fs::write(&src_file, "// source").unwrap();

        for companion in &["requirements.md", "tasks.md", "context.md", "testing.md"] {
            fs::write(specs_dir.join(companion), "---\nspec: mymod.spec.md\n---\n").unwrap();
        }

        let spec_content = r#"---
module: mymod
version: 1
status: active
files:
  - src/mymod.rs
db_tables: []
depends_on: []
---

# Mymod

## Purpose
Test

## Public API
Test

## Invariants
Test

## Behavioral Examples
Test

## Error Cases
Test

## Dependencies
Test

## Change Log
Test
"#;
        let spec_path = specs_dir.join("mymod.spec.md");
        fs::write(&spec_path, spec_content).unwrap();

        let required = vec![
            "Purpose".to_string(),
            "Public API".to_string(),
            "Invariants".to_string(),
            "Behavioral Examples".to_string(),
            "Error Cases".to_string(),
            "Dependencies".to_string(),
            "Change Log".to_string(),
        ];

        let result = validate_spec(&spec_path, tmp.path(), &required);
        assert_eq!(result.name, "mymod");
        assert_eq!(result.version, 1);
        assert_eq!(result.status, "active");
        assert!(!result.has_errors());
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_validate_spec_missing_file() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path().join("specs/mymod");
        fs::create_dir_all(&specs_dir).unwrap();

        let spec_content = r#"---
module: mymod
version: 1
status: active
files:
  - src/nonexistent.rs
db_tables: []
depends_on: []
---

# Mymod

## Purpose
## Public API
## Invariants
## Behavioral Examples
## Error Cases
## Dependencies
## Change Log
"#;
        let spec_path = specs_dir.join("mymod.spec.md");
        fs::write(&spec_path, spec_content).unwrap();

        let required = vec![
            "Purpose".to_string(),
            "Public API".to_string(),
            "Invariants".to_string(),
            "Behavioral Examples".to_string(),
            "Error Cases".to_string(),
            "Dependencies".to_string(),
            "Change Log".to_string(),
        ];

        let result = validate_spec(&spec_path, tmp.path(), &required);
        assert!(result.has_errors());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("file not found")));
    }

    #[test]
    fn test_validate_spec_missing_sections() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path().join("specs/mymod");
        fs::create_dir_all(&specs_dir).unwrap();

        let spec_content = r#"---
module: mymod
version: 1
status: active
files: []
db_tables: []
depends_on: []
---

# Mymod

## Purpose
Test

## Public API
Test
"#;
        let spec_path = specs_dir.join("mymod.spec.md");
        fs::write(&spec_path, spec_content).unwrap();

        let required = vec![
            "Purpose".to_string(),
            "Public API".to_string(),
            "Invariants".to_string(),
        ];

        let result = validate_spec(&spec_path, tmp.path(), &required);
        assert!(result.has_errors());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("Invariants")));
    }

    #[test]
    fn test_validate_spec_missing_companion() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path().join("specs/mymod");
        fs::create_dir_all(&specs_dir).unwrap();

        let spec_content = r#"---
module: mymod
version: 1
status: active
files: []
db_tables: []
depends_on: []
---

# Mymod

## Purpose
## Public API
## Invariants
## Behavioral Examples
## Error Cases
## Dependencies
## Change Log
"#;
        let spec_path = specs_dir.join("mymod.spec.md");
        fs::write(&spec_path, spec_content).unwrap();

        let required = vec![
            "Purpose".to_string(),
            "Public API".to_string(),
            "Invariants".to_string(),
            "Behavioral Examples".to_string(),
            "Error Cases".to_string(),
            "Dependencies".to_string(),
            "Change Log".to_string(),
        ];

        let result = validate_spec(&spec_path, tmp.path(), &required);
        assert!(!result.has_errors());
        assert!(result.has_warnings());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("companion file missing")));
    }

    #[test]
    fn test_validate_spec_invalid_status() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path().join("specs/mymod");
        fs::create_dir_all(&specs_dir).unwrap();

        let spec_content = r#"---
module: mymod
version: 1
status: banana
files: []
db_tables: []
depends_on: []
---

# Mymod

## Purpose
## Public API
## Invariants
## Behavioral Examples
## Error Cases
## Dependencies
## Change Log
"#;
        let spec_path = specs_dir.join("mymod.spec.md");
        fs::write(&spec_path, spec_content).unwrap();

        let required = vec![
            "Purpose".to_string(),
            "Public API".to_string(),
            "Invariants".to_string(),
            "Behavioral Examples".to_string(),
            "Error Cases".to_string(),
            "Dependencies".to_string(),
            "Change Log".to_string(),
        ];

        let result = validate_spec(&spec_path, tmp.path(), &required);
        assert!(result.has_errors());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("Invalid status")));
    }

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("hello_world"), "Hello World");
        assert_eq!(to_title_case("auth"), "Auth");
        assert_eq!(to_title_case("create_template"), "Create Template");
    }

    #[test]
    fn test_init_creates_files() {
        let tmp = TempDir::new().unwrap();

        let result = init(tmp.path());

        assert!(result.is_ok());
        assert!(tmp.path().join(".specsync/config.toml").exists());
        assert!(tmp.path().join(".specsync/registry.toml").exists());
        assert!(tmp.path().join(".specsync/.gitignore").exists());
        assert!(tmp.path().join(".specsync/version").exists());
        assert!(tmp.path().join("specs").exists());
    }

    #[test]
    fn test_init_refuses_existing() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".specsync")).unwrap();

        let result = init(tmp.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_new_spec_creates_files() {
        let tmp = TempDir::new().unwrap();

        let specsync_dir = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync_dir).unwrap();
        fs::write(
            specsync_dir.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();
        fs::write(
            specsync_dir.join("registry.toml"),
            "[registry]\nname = \"test\"\n\n[specs]\n",
        )
        .unwrap();

        let result = new_spec(tmp.path(), "auth");

        assert!(result.is_ok());
        assert!(tmp.path().join("specs/auth/auth.spec.md").exists());
        assert!(tmp.path().join("specs/auth/requirements.md").exists());
        assert!(tmp.path().join("specs/auth/tasks.md").exists());
        assert!(tmp.path().join("specs/auth/context.md").exists());
        assert!(tmp.path().join("specs/auth/testing.md").exists());

        let registry = fs::read_to_string(specsync_dir.join("registry.toml")).unwrap();
        assert!(registry.contains("auth = \"specs/auth/auth.spec.md\""));
    }

    #[test]
    fn test_new_spec_refuses_existing() {
        let tmp = TempDir::new().unwrap();

        let specsync_dir = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync_dir).unwrap();
        fs::write(
            specsync_dir.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();

        fs::create_dir_all(tmp.path().join("specs/auth")).unwrap();

        let result = new_spec(tmp.path(), "auth");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_spec_result_counts() {
        let result = SpecResult {
            name: "test".to_string(),
            version: 1,
            status: "active".to_string(),
            file_count: 1,
            section_count: 7,
            required_count: 7,
            issues: vec![
                ValidationIssue {
                    message: "error1".to_string(),
                    is_error: true,
                },
                ValidationIssue {
                    message: "warn1".to_string(),
                    is_error: false,
                },
                ValidationIssue {
                    message: "warn2".to_string(),
                    is_error: false,
                },
            ],
        };
        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 2);
        assert!(result.has_errors());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_parse_frontmatter_inline_files() {
        let content = r#"---
module: test
version: 1
status: draft
files: [src/a.rs, src/b.rs]
db_tables: []
depends_on: []
---

body
"#;
        let (fm, _) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.files, vec!["src/a.rs", "src/b.rs"]);
    }
}
