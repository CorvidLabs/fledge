use anyhow::Result;
use console::style;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tera::Tera;
use walkdir::WalkDir;

use crate::templates::{matches_glob_pub, TemplateManifest};

/// JSON schema version for the templates `validate` envelope (single and multi
/// share the same `{schema_version, reports}` shape). See lanes.rs for the
/// per-command rationale.
pub const VALIDATE_SCHEMA: u32 = 1;

pub struct ValidateOptions {
    pub path: PathBuf,
    pub strict: bool,
    pub json: bool,
}

#[derive(Default, serde::Serialize)]
struct ValidationReport {
    template: String,
    path: String,
    errors: Vec<String>,
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    missing_requirements: Vec<String>,
}

const BUILTIN_VARS: &[&str] = &[
    "project_name",
    "project_name_snake",
    "project_name_kebab",
    "project_name_pascal",
    "project_name_camel",
    "year",
    "date",
    "author",
    "github_org",
    "license",
];

pub fn run(opts: ValidateOptions) -> Result<()> {
    let path = opts.path.canonicalize().unwrap_or(opts.path.clone());

    if path.is_dir() && path.join("template.toml").exists() {
        let report = validate_single(&path)?;
        return print_report(&report, opts.strict, opts.json);
    }

    if path.is_dir() {
        let mut reports = Vec::new();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let sub = entry.path();
            if sub.is_dir() && sub.join("template.toml").exists() {
                reports.push(validate_single(&sub)?);
            }
        }
        if reports.is_empty() {
            anyhow::bail!(
                "No templates found in '{}'. Each template needs a template.toml.",
                path.display()
            );
        }
        return print_reports(&reports, opts.strict, opts.json);
    }

    anyhow::bail!(
        "'{}' is not a directory. Point to a template directory or a directory of templates.",
        path.display()
    );
}

fn validate_single(path: &Path) -> Result<ValidationReport> {
    let mut report = ValidationReport {
        path: path.display().to_string(),
        ..Default::default()
    };

    let manifest_path = path.join("template.toml");
    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            report
                .errors
                .push(format!("Cannot read template.toml: {e}"));
            return Ok(report);
        }
    };

    let manifest: TemplateManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            report.errors.push(format!("Invalid template.toml: {e}"));
            return Ok(report);
        }
    };

    report.template = manifest.template.name.clone();

    if manifest.template.name.is_empty() {
        report.errors.push("template.name is empty".to_string());
    }
    if manifest.template.description.is_empty() {
        report
            .errors
            .push("template.description is empty".to_string());
    }

    if !manifest.template.requires.is_empty() {
        let (_, missing) = crate::templates::check_requirements(&manifest.template.requires);
        if !missing.is_empty() {
            for tool in &missing {
                report
                    .warnings
                    .push(format!("required tool '{tool}' not found on PATH"));
            }
            report.missing_requirements = missing;
        }
    }

    let known_vars: HashSet<String> = BUILTIN_VARS
        .iter()
        .map(|s| s.to_string())
        .chain(manifest.prompts.keys().cloned())
        .collect();

    let ignore_set: Vec<&str> = manifest.files.ignore.iter().map(|s| s.as_str()).collect();

    let mut file_count = 0;

    for entry in WalkDir::new(path).min_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                report.warnings.push(format!("Walk error: {e}"));
                continue;
            }
        };

        let rel_path = entry.path().strip_prefix(path).unwrap_or(entry.path());
        let rel_str = rel_path.to_string_lossy();

        if ignore_set.iter().any(|ig| matches_glob_pub(ig, &rel_str)) {
            continue;
        }

        if rel_str == "template.toml" {
            continue;
        }

        if entry.file_type().is_dir() {
            continue;
        }

        file_count += 1;

        let rel_string = rel_str.to_string();
        let is_tera_ext = rel_string.ends_with(".tera");
        let should_render = is_tera_ext
            || manifest
                .files
                .render
                .iter()
                .any(|g| matches_glob_pub(g, &rel_string));

        if should_render {
            let file_content = match std::fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut tera = Tera::default();
            if let Err(e) = tera.add_raw_template("__validate__", &file_content) {
                report
                    .errors
                    .push(format!("{rel_str}: Tera syntax error: {e}"));
                continue;
            }

            for var in extract_variables(&file_content) {
                if !known_vars.contains(&var) {
                    report.warnings.push(format!(
                        "{rel_str}: uses undefined variable '{var}' (may need a prompt)"
                    ));
                }
            }
        }

        if rel_string.contains("{{") {
            let output_rel = if is_tera_ext {
                rel_string.trim_end_matches(".tera").to_string()
            } else {
                rel_string.clone()
            };
            let mut tera = Tera::default();
            if let Err(e) = tera.add_raw_template("__path__", &output_rel) {
                report
                    .errors
                    .push(format!("Path '{rel_str}': Tera syntax error: {e}"));
            } else {
                for var in extract_variables(&output_rel) {
                    if !known_vars.contains(&var) {
                        report
                            .warnings
                            .push(format!("Path '{rel_str}': uses undefined variable '{var}'"));
                    }
                }
            }
        }
    }

    if file_count == 0 {
        report
            .warnings
            .push("Template has no files (besides template.toml)".to_string());
    }

    for glob in &manifest.files.render {
        let matched = WalkDir::new(path)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .any(|e| {
                let rel = e.path().strip_prefix(path).unwrap_or(e.path());
                let rel_str = rel.to_string_lossy();
                if rel_str == "template.toml" {
                    return false;
                }
                let effective = if rel_str.ends_with(".tera") {
                    rel_str.trim_end_matches(".tera").to_string()
                } else {
                    rel_str.to_string()
                };
                matches_glob_pub(glob, &rel_str) || matches_glob_pub(glob, &effective)
            });
        if !matched {
            report.warnings.push(format!(
                "files.render glob '{glob}' doesn't match any files"
            ));
        }
    }

    if !manifest.files.ignore.iter().any(|i| i == "template.toml") {
        report.warnings.push(
            "files.ignore doesn't include 'template.toml' — it will be copied to output"
                .to_string(),
        );
    }

    Ok(report)
}

fn extract_variables(content: &str) -> HashSet<String> {
    let re = regex_lite::Regex::new(r"\{\{[\s]*([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let dollar_re = regex_lite::Regex::new(r"\$\{\{").unwrap();
    let dollar_positions: HashSet<usize> = dollar_re
        .find_iter(content)
        .map(|m| m.start() + 1)
        .collect();
    re.captures_iter(content)
        .filter(|cap| {
            let start = cap.get(0).unwrap().start();
            !dollar_positions.contains(&start)
        })
        .map(|cap| cap.get(1).unwrap().as_str().to_string())
        .collect()
}

fn print_report(report: &ValidationReport, strict: bool, json: bool) -> Result<()> {
    if json {
        let result = serde_json::json!({
            "schema_version": VALIDATE_SCHEMA,
            "reports": [report],
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return check_result(std::slice::from_ref(report), strict);
    }

    let name = if report.template.is_empty() {
        &report.path
    } else {
        &report.template
    };

    if report.errors.is_empty() && report.warnings.is_empty() {
        println!(
            "{} {} — valid",
            style("✅").green().bold(),
            style(name).green()
        );
    } else {
        println!("{}", style(name).bold());
        for e in &report.errors {
            println!("  {} {}", style("error:").red().bold(), e);
        }
        for w in &report.warnings {
            println!("  {} {}", style("warn:").yellow().bold(), w);
        }
    }

    check_result(std::slice::from_ref(report), strict)
}

fn print_reports(reports: &[ValidationReport], strict: bool, json: bool) -> Result<()> {
    if json {
        let result = serde_json::json!({
            "schema_version": VALIDATE_SCHEMA,
            "reports": reports,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return check_result(reports, strict);
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;

    for report in reports {
        let name = if report.template.is_empty() {
            &report.path
        } else {
            &report.template
        };

        if report.errors.is_empty() && report.warnings.is_empty() {
            println!("  {} {}", style("✅").green().bold(), style(name).green());
        } else {
            println!("  {} {}", style("❌").red().bold(), style(name).red());
            for e in &report.errors {
                println!("    {} {}", style("error:").red().bold(), e);
            }
            for w in &report.warnings {
                println!("    {} {}", style("warn:").yellow().bold(), w);
            }
        }

        total_errors += report.errors.len();
        total_warnings += report.warnings.len();
    }

    println!(
        "\n{} templates, {} errors, {} warnings",
        reports.len(),
        total_errors,
        total_warnings
    );

    check_result(reports, strict)
}

fn check_result(reports: &[ValidationReport], strict: bool) -> Result<()> {
    let has_errors = reports.iter().any(|r| !r.errors.is_empty());
    let has_warnings = reports.iter().any(|r| !r.warnings.is_empty());

    if has_errors || (strict && has_warnings) {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_template(dir: &Path, manifest: &str, files: &[(&str, &str)]) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("template.toml"), manifest).unwrap();
        for (path, content) in files {
            let file_path = dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }
    }

    #[test]
    fn valid_template_passes() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("my-tpl");
        make_template(
            &tpl,
            r#"
[template]
name = "my-tpl"
description = "A test template"

[files]
render = ["**/*.rs"]
ignore = ["template.toml"]
"#,
            &[(
                "src/main.rs",
                "fn main() { println!(\"{{ project_name }}\"); }",
            )],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    }

    #[test]
    fn missing_manifest_is_error() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("broken");
        fs::create_dir_all(&tpl).unwrap();
        // No template.toml

        let report = validate_single(&tpl).unwrap();
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("Cannot read template.toml"));
    }

    #[test]
    fn invalid_toml_is_error() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("bad-toml");
        make_template(&tpl, "not valid toml {{{}}", &[]);

        let report = validate_single(&tpl).unwrap();
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("Invalid template.toml"));
    }

    #[test]
    fn empty_name_is_error() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("empty-name");
        make_template(
            &tpl,
            r#"
[template]
name = ""
description = "Has no name"

[files]
ignore = ["template.toml"]
"#,
            &[("file.txt", "hello")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.iter().any(|e| e.contains("name is empty")));
    }

    #[test]
    fn broken_tera_syntax_is_error() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("bad-tera");
        make_template(
            &tpl,
            r#"
[template]
name = "bad-tera"
description = "Broken tera"

[files]
render = ["**/*.txt"]
ignore = ["template.toml"]
"#,
            &[("file.txt", "Hello {{ broken unclosed")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("Tera syntax error")),
            "errors: {:?}",
            report.errors
        );
    }

    #[test]
    fn tera_extension_validated() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("tera-ext");
        make_template(
            &tpl,
            r#"
[template]
name = "tera-ext"
description = "Tera extension"

[files]
ignore = ["template.toml"]
"#,
            &[("README.md.tera", "# {{ project_name }}")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    }

    #[test]
    fn undefined_variable_is_warning() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("undef-var");
        make_template(
            &tpl,
            r#"
[template]
name = "undef-var"
description = "Undefined var"

[files]
render = ["**/*.txt"]
ignore = ["template.toml"]
"#,
            &[("file.txt", "Hello {{ custom_undefined_var }}")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("undefined variable")),
            "warnings: {:?}",
            report.warnings
        );
    }

    #[test]
    fn custom_prompt_var_passes() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("custom-var");
        make_template(
            &tpl,
            r#"
[template]
name = "custom-var"
description = "Custom var"

[prompts.go_module]
message = "Go module path"
default = "github.com/example/{{ project_name }}"

[files]
render = ["**/*.go"]
ignore = ["template.toml"]
"#,
            &[("main.go", "package main // {{ go_module }}")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert!(
            !report.warnings.iter().any(|w| w.contains("go_module")),
            "should not warn for defined prompt var"
        );
    }

    #[test]
    fn missing_template_toml_ignore_warns() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("no-ignore");
        make_template(
            &tpl,
            r#"
[template]
name = "no-ignore"
description = "No ignore"

[files]
render = []
"#,
            &[("file.txt", "hello")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(
            report.warnings.iter().any(|w| w.contains("template.toml")),
            "should warn about template.toml not ignored"
        );
    }

    #[test]
    fn unmatched_render_glob_warns() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("orphan-glob");
        make_template(
            &tpl,
            r#"
[template]
name = "orphan-glob"
description = "Orphan glob"

[files]
render = ["**/*.py"]
ignore = ["template.toml"]
"#,
            &[("file.txt", "hello")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(
            report.warnings.iter().any(|w| w.contains("doesn't match")),
            "warnings: {:?}",
            report.warnings
        );
    }

    #[test]
    fn batch_validation_finds_all() {
        let tmp = TempDir::new().unwrap();

        make_template(
            &tmp.path().join("tpl-a"),
            r#"
[template]
name = "tpl-a"
description = "Template A"

[files]
ignore = ["template.toml"]
"#,
            &[("a.txt", "hello")],
        );

        make_template(
            &tmp.path().join("tpl-b"),
            r#"
[template]
name = "tpl-b"
description = "Template B"

[files]
ignore = ["template.toml"]
"#,
            &[("b.txt", "world")],
        );

        let result = run(ValidateOptions {
            path: tmp.path().to_path_buf(),
            strict: false,
            json: false,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn missing_requirement_is_warning() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("needs-tool");
        make_template(
            &tpl,
            r#"
[template]
name = "needs-tool"
description = "Needs a nonexistent tool"
requires = ["fledge_nonexistent_tool_xyz"]

[files]
ignore = ["template.toml"]
"#,
            &[("file.txt", "hello")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("fledge_nonexistent_tool_xyz")),
            "should warn about missing tool"
        );
        assert_eq!(
            report.missing_requirements,
            vec!["fledge_nonexistent_tool_xyz"]
        );
    }

    #[test]
    fn present_requirement_no_warning() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("needs-sh");
        make_template(
            &tpl,
            r#"
[template]
name = "needs-sh"
description = "Needs sh which always exists"
requires = ["sh"]

[files]
ignore = ["template.toml"]
"#,
            &[("file.txt", "hello")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.is_empty());
        assert!(
            !report.warnings.iter().any(|w| w.contains("not found")),
            "should not warn about sh: {:?}",
            report.warnings
        );
        assert!(report.missing_requirements.is_empty());
    }

    #[test]
    fn requires_field_is_optional() {
        let tmp = TempDir::new().unwrap();
        let tpl = tmp.path().join("no-requires");
        make_template(
            &tpl,
            r#"
[template]
name = "no-requires"
description = "No requires field at all"

[files]
ignore = ["template.toml"]
"#,
            &[("file.txt", "hello")],
        );

        let report = validate_single(&tpl).unwrap();
        assert!(report.errors.is_empty());
        assert!(report.missing_requirements.is_empty());
    }
}
