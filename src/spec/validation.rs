use std::fs;
use std::path::Path;

use super::{parse, COMPANION_FILES};

#[derive(Debug)]
pub(crate) struct ValidationIssue {
    pub(crate) message: String,
    pub(crate) is_error: bool,
}

#[derive(Debug)]
pub(crate) struct SpecResult {
    pub(crate) name: String,
    pub(crate) version: u32,
    pub(crate) status: String,
    pub(crate) file_count: usize,
    pub(crate) section_count: usize,
    pub(crate) required_count: usize,
    pub(crate) issues: Vec<ValidationIssue>,
}

impl SpecResult {
    pub(crate) fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.is_error)
    }

    pub(crate) fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| !i.is_error)
    }

    pub(crate) fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.is_error).count()
    }

    pub(crate) fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| !i.is_error).count()
    }
}

pub(crate) fn validate_spec(
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

    let (fm, body) = match parse::parse_frontmatter(&content) {
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

    let sections = parse::extract_sections(&body);
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
