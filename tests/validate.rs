mod common;
use common::*;

use std::fs;
use tempfile::TempDir;

// Validate-template command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_validate_template_valid() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("my-tpl");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
name = "my-tpl"
description = "A test template"

[files]
render = ["**/*.rs"]
ignore = ["template.toml"]
"#,
    )
    .unwrap();
    fs::create_dir_all(tpl.join("src")).unwrap();
    fs::write(
        tpl.join("src/main.rs"),
        "fn main() { println!(\"{{ project_name }}\"); }",
    )
    .unwrap();

    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn cli_validate_template_invalid_toml_fails() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("bad");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(tpl.join("template.toml"), "not valid {{{}}\n").unwrap();

    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_json_output() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("json-tpl");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
name = "json-tpl"
description = "JSON output test"

[files]
ignore = ["template.toml"]
"#,
    )
    .unwrap();
    fs::write(tpl.join("file.txt"), "hello").unwrap();

    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap(), "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn cli_validate_template_strict_fails_on_warnings() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("warn-tpl");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
name = "warn-tpl"
description = "Has warnings"

[files]
render = ["**/*.py"]
"#,
    )
    .unwrap();
    fs::write(tpl.join("file.txt"), "hello").unwrap();

    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap(), "--strict"]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_batch_directory() {
    let tmp = TempDir::new().unwrap();

    for name in &["tpl-a", "tpl-b"] {
        let tpl = tmp.path().join(name);
        fs::create_dir_all(&tpl).unwrap();
        fs::write(
            tpl.join("template.toml"),
            format!(
                r#"[template]
name = "{name}"
description = "Batch test"

[files]
ignore = ["template.toml"]
"#
            ),
        )
        .unwrap();
        fs::write(tpl.join("file.txt"), "hello").unwrap();
    }

    let output = run_fledge(&["templates", "validate", tmp.path().to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn cli_validate_template_no_templates_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&["templates", "validate", tmp.path().to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No templates found"));
}

// ──────────────────────────────────────────────────────────
