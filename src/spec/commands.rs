use anyhow::{bail, Context, Result};
use console::style;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::{
    classify_companions, find_spec_files, load_config, parse, to_title_case, validate_module_name,
    validation, SPEC_CHECK_SCHEMA, SPEC_LIST_SCHEMA, SPEC_SHOW_SCHEMA,
};

#[derive(Debug, Serialize)]
pub(crate) struct SpecSummary {
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
pub(crate) struct SpecDetail {
    name: String,
    version: u32,
    status: String,
    path: String,
    files: Vec<String>,
    sections: Vec<String>,
    companions: Vec<String>,
    missing_companions: Vec<String>,
}

pub(crate) fn check(root: &Path, strict: bool, json: bool) -> Result<()> {
    let config = load_config(root)?;
    let specs_dir = root.join(config.specs_dir.as_deref().unwrap_or("specs"));

    if !specs_dir.exists() {
        if json {
            let payload = serde_json::json!({
                "schema_version": SPEC_CHECK_SCHEMA,
                "action": "spec_check",
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

    let mut results: Vec<validation::SpecResult> = Vec::new();

    for entry in WalkDir::new(&specs_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".spec.md") {
                results.push(validation::validate_spec(path, root, &required_sections));
            }
        }
    }

    if results.is_empty() {
        if json {
            let payload = serde_json::json!({
                "schema_version": SPEC_CHECK_SCHEMA,
                "action": "spec_check",
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
            "schema_version": SPEC_CHECK_SCHEMA,
            "action": "spec_check",
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

pub(crate) fn build_summary(
    spec_path: &Path,
    root: &Path,
    required_count: usize,
) -> Result<SpecSummary> {
    let content = fs::read_to_string(spec_path)
        .with_context(|| format!("reading {}", spec_path.display()))?;
    let (fm, body) = parse::parse_frontmatter(&content)
        .with_context(|| format!("parsing frontmatter in {}", spec_path.display()))?;
    let sections = parse::extract_sections(&body);
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

pub(crate) fn list_specs(root: &Path, json: bool) -> Result<()> {
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
        let envelope = serde_json::json!({
            "schema_version": SPEC_LIST_SCHEMA,
            "action": "spec_list",
            "specs": summaries,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
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

pub(crate) fn show_spec(root: &Path, name: &str, json: bool) -> Result<()> {
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
    let (fm, body) = parse::parse_frontmatter(&content)
        .with_context(|| format!("parsing frontmatter in {}", spec_path.display()))?;
    let sections = parse::extract_sections(&body);
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
        let envelope = serde_json::json!({
            "schema_version": SPEC_SHOW_SCHEMA,
            "action": "spec_show",
            "spec": detail,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
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

pub(crate) fn init(root: &Path) -> Result<()> {
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

pub(crate) fn new_spec(root: &Path, name: &str) -> Result<()> {
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
