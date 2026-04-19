use anyhow::{Context, Result, bail};
use console::style;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    #[serde(default)]
    #[allow(dead_code)]
    pub db_tables: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub depends_on: Vec<String>,
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
        SpecAction::Check { strict } => check(&root, strict),
        SpecAction::Init => init(&root),
        SpecAction::New { name } => new_spec(&root, &name),
    }
}

#[derive(Debug)]
pub enum SpecAction {
    Check { strict: bool },
    Init,
    New { name: String },
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
    let mut db_tables = Vec::new();
    let mut depends_on = Vec::new();
    let mut current_list: Option<&str> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- ") {
            let value = rest.trim().to_string();
            match current_list {
                Some("files") => files.push(value),
                Some("db_tables") => db_tables.push(value),
                Some("depends_on") => depends_on.push(value),
                _ => {}
            }
            continue;
        }

        current_list = None;

        if let Some((key, val)) = trimmed.split_once(':') {
            let key = key.trim();
            let val = val.trim();

            if val.is_empty() || val == "[]" {
                match key {
                    "files" => {
                        if val == "[]" {
                            files.clear();
                        } else {
                            current_list = Some("files");
                        }
                    }
                    "db_tables" => {
                        if val == "[]" {
                            db_tables.clear();
                        } else {
                            current_list = Some("db_tables");
                        }
                    }
                    "depends_on" => {
                        if val == "[]" {
                            depends_on.clear();
                        } else {
                            current_list = Some("depends_on");
                        }
                    }
                    _ => {}
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
        db_tables,
        depends_on,
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
    let companion_files = ["requirements.md", "tasks.md", "context.md", "testing.md"];
    for companion in &companion_files {
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

fn check(root: &Path, strict: bool) -> Result<()> {
    let config = load_config(root)?;
    let specs_dir = root.join(config.specs_dir.as_deref().unwrap_or("specs"));

    if !specs_dir.exists() {
        println!(
            "{} No specs directory found at {}",
            style("*").cyan().bold(),
            style(specs_dir.display()).dim()
        );
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
        println!(
            "{} No spec files found in {}",
            style("*").cyan().bold(),
            style(specs_dir.display()).dim()
        );
        return Ok(());
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));

    let mut total_errors = 0;
    let mut total_warnings = 0;

    for result in &results {
        let errors = result.error_count();
        let warnings = result.warning_count();
        total_errors += errors;
        total_warnings += warnings;

        if result.has_errors() || (strict && result.has_warnings()) {
            print!(
                "{} {} (v{}, {})",
                style("✗").red().bold(),
                style(&result.name).red(),
                result.version,
                result.status,
            );
        } else if result.has_warnings() {
            print!(
                "{} {} (v{}, {})",
                style("⚠").yellow().bold(),
                style(&result.name).yellow(),
                result.version,
                result.status,
            );
        } else {
            print!(
                "{} {} (v{}, {})",
                style("✓").green().bold(),
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
        style("✓").green().bold()
    );

    fs::write(specsync_dir.join("registry.toml"), registry_content)?;
    println!(
        "{} Created .specsync/registry.toml",
        style("✓").green().bold()
    );

    fs::write(specsync_dir.join(".gitignore"), gitignore_content)?;
    println!("{} Created .specsync/.gitignore", style("✓").green().bold());

    fs::write(specsync_dir.join("version"), "4.3.1\n")?;
    println!("{} Created .specsync/version", style("✓").green().bold());

    if !specs_dir.exists() {
        fs::create_dir_all(&specs_dir)?;
        println!("{} Created specs/", style("✓").green().bold());
    }

    println!();
    println!(
        "  Spec-sync initialized. Run {} to create your first spec.",
        style("fledge spec new <name>").cyan()
    );

    Ok(())
}

fn new_spec(root: &Path, name: &str) -> Result<()> {
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

TODO: Describe the purpose of this module.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|

### Structs & Enums

| Type | Description |
|------|-------------|

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|

## Invariants

1. TODO

## Behavioral Examples

```
TODO: Add behavioral examples
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|

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

- As a developer, I want to TODO

## Acceptance Criteria

- TODO

## Constraints

- TODO

## Out of Scope

- TODO
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

TODO: Describe the context and motivation for this module.

## Related Modules

- TODO

## Design Decisions

- TODO
"#
    );

    let testing_content = format!(
        r#"---
spec: {name}.spec.md
---

## Test Plan

### Unit Tests

- TODO

### Integration Tests

- TODO
"#
    );

    fs::write(spec_dir.join(format!("{name}.spec.md")), &spec_content)?;
    println!(
        "{} Created specs/{name}/{name}.spec.md",
        style("✓").green().bold()
    );

    fs::write(spec_dir.join("requirements.md"), &requirements_content)?;
    println!(
        "{} Created specs/{name}/requirements.md",
        style("✓").green().bold()
    );

    fs::write(spec_dir.join("tasks.md"), &tasks_content)?;
    println!(
        "{} Created specs/{name}/tasks.md",
        style("✓").green().bold()
    );

    fs::write(spec_dir.join("context.md"), &context_content)?;
    println!(
        "{} Created specs/{name}/context.md",
        style("✓").green().bold()
    );

    fs::write(spec_dir.join("testing.md"), &testing_content)?;
    println!(
        "{} Created specs/{name}/testing.md",
        style("✓").green().bold()
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
        assert!(fm.db_tables.is_empty());
        assert_eq!(fm.depends_on, vec!["templates"]);
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
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("file not found"))
        );
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
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("Invariants"))
        );
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
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("companion file missing"))
        );
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
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("Invalid status"))
        );
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
