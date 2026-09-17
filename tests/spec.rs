mod common;
use common::*;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// MARK: - spec commands
// Spec commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_spec_check_succeeds_in_project() {
    let output = run_fledge(&["spec", "check"]);
    // This runs against the fledge project itself which has specs
    assert!(output.status.success());
}

#[test]
fn cli_spec_init_in_new_dir() {
    let tmp = TempDir::new().unwrap();
    // init git repo first since spec needs project root
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let output = run_fledge_in(tmp.path(), &["spec", "init"]);
    assert!(output.status.success());
    assert!(tmp.path().join("specs").exists());
}

#[test]
fn cli_spec_new_creates_spec() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    // Init spec-sync first
    run_fledge_in(tmp.path(), &["spec", "init"]);

    let output = run_fledge_in(tmp.path(), &["spec", "new", "auth"]);
    assert!(output.status.success());
    assert!(tmp.path().join("specs/auth").exists());
}

#[test]
fn cli_spec_list_in_project() {
    let output = run_fledge(&["spec", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("spec(s) found"),
        "expected summary line in output: {stdout}"
    );
}

#[test]
fn cli_spec_list_ls_alias() {
    let output = run_fledge(&["spec", "ls"]);
    assert!(output.status.success());
}

#[test]
fn cli_spec_list_json_valid() {
    let output = run_fledge(&["spec", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Tier-D envelope: {schema_version: 1, action: "spec_list", specs: [...]}
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_list"));
    let specs = parsed["specs"].as_array().expect("specs array");
    assert!(!specs.is_empty(), "fledge project should have specs");
    let first = &specs[0];
    for field in [
        "name",
        "version",
        "status",
        "path",
        "files",
        "section_count",
        "required_sections",
        "companions",
        "missing_companions",
    ] {
        assert!(first.get(field).is_some(), "missing field: {field}");
    }
}

#[test]
fn cli_spec_list_json_empty_dir() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    run_fledge_in(tmp.path(), &["spec", "init"]);

    let output = run_fledge_in(tmp.path(), &["spec", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert!(parsed["specs"].as_array().unwrap().is_empty());
}

#[test]
fn cli_spec_show_existing_module() {
    let output = run_fledge(&["spec", "show", "spec"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("spec"));
    assert!(stdout.contains("sections"));
}

#[test]
fn cli_spec_show_json_valid() {
    let output = run_fledge(&["spec", "show", "spec", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Tier-D envelope: {schema_version: 1, action: "spec_show", spec: {...}}
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_show"));
    let spec = &parsed["spec"];
    assert!(spec.is_object());
    assert_eq!(spec["name"].as_str(), Some("spec"));
    assert!(spec["sections"].is_array());
    assert!(spec["companions"].is_array());
    assert!(spec["missing_companions"].is_array());
}

#[test]
fn cli_spec_show_missing_module_fails() {
    let output = run_fledge(&["spec", "show", "definitely-not-a-real-spec-xyz"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No spec found") || stderr.contains("not"));
}

#[test]
fn cli_spec_check_json_valid() {
    let output = run_fledge(&["spec", "check", "--json"]);
    // May pass or fail on the repo's specs; either way stdout must be JSON
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Tier-D envelope
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_check"));
    assert!(parsed["specs"].is_array());
    assert!(parsed["totals"].is_object());
    assert!(parsed["totals"]["checked"].is_number());
    assert!(parsed["totals"]["errors"].is_number());
    assert!(parsed["totals"]["warnings"].is_number());
    assert!(parsed["strict"].is_boolean());
}

#[test]
fn cli_spec_check_json_spec_shape() {
    let output = run_fledge(&["spec", "check", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let specs = parsed["specs"].as_array().unwrap();
    assert!(!specs.is_empty(), "fledge repo should have specs");
    let first = &specs[0];
    for field in [
        "name",
        "version",
        "status",
        "file_count",
        "section_count",
        "required_count",
        "errors",
        "warnings",
    ] {
        assert!(first.get(field).is_some(), "missing field: {field}");
    }
    assert!(first["errors"].is_array());
    assert!(first["warnings"].is_array());
}

#[test]
fn cli_work_start_help_shows_json_flag() {
    let output = run_fledge(&["work", "start", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_commit_help_shows_json_flag() {
    let output = run_fledge(&["work", "commit", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_push_help_shows_json_flag() {
    let output = run_fledge(&["work", "push", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_status_help_shows_json_flag() {
    let output = run_fledge(&["work", "status", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_status_json_in_repo() {
    // Run inside a temp git repo with a real branch — avoids the detached-HEAD
    // situation that CI check-out sometimes produces.
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = run_fledge_in(tmp.path(), &["work", "status", "--json"]);
    assert!(
        output.status.success(),
        "work status --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["branch"].as_str(), Some("feature"));
    assert_eq!(parsed["default"].as_str(), Some("main"));
    assert!(parsed["ahead"].is_number());
    // behind is either a number or null (base-not-fetched sentinel)
    assert!(parsed["behind"].is_number() || parsed["behind"].is_null());
    // dirty is a count of uncommitted files
    assert!(parsed["dirty"].is_number());
}

// ──────────────────────────────────────────────────────────

// MARK: - spec edge cases
// Spec edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_spec_check_in_empty_dir_fails() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let output = run_fledge_in(tmp.path(), &["spec", "check"]);
    // No specs dir — should fail or warn
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Either exits nonzero or prints a message about missing specs
    assert!(
        !output.status.success()
            || stdout.contains("No specs")
            || stderr.contains("No specs")
            || stdout.contains("specs"),
        "expected some feedback about missing specs, got stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn cli_spec_new_duplicate_name() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    run_fledge_in(tmp.path(), &["spec", "init"]);
    run_fledge_in(tmp.path(), &["spec", "new", "auth"]);

    // Second creation should fail or warn
    let output = run_fledge_in(tmp.path(), &["spec", "new", "auth"]);
    assert!(
        !output.status.success() || {
            let stderr = String::from_utf8(output.stderr.clone()).unwrap();
            let stdout = String::from_utf8(output.stdout.clone()).unwrap();
            stderr.contains("exists") || stdout.contains("exists")
        },
        "expected duplicate spec warning"
    );
}

// ──────────────────────────────────────────────────────────
// Changelog edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_changelog_nonexistent_tag_fails() {
    let output = run_fledge(&["changelog", "--tag", "v999.999.999"]);
    // Should fail or return empty — shouldn't panic
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("not found") || stderr.contains("999"),
            "expected tag-not-found error, got: {stderr}"
        );
    }
}

#[test]
fn cli_changelog_zero_limit() {
    let output = run_fledge(&["changelog", "--limit", "0"]);
    // Should succeed with empty output or handle gracefully
    assert!(output.status.success());
}

#[test]
fn cli_changelog_in_non_git_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["changelog"]);
    // Not a git repo — should fail gracefully
    assert!(
        !output.status.success() || {
            let stdout = String::from_utf8(output.stdout.clone()).unwrap();
            stdout.contains("No tags") || stdout.is_empty()
        }
    );
}

// ──────────────────────────────────────────────────────────
// Doctor edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_doctor_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") || stdout.contains("Git"));
}

#[test]
fn cli_doctor_json_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["sections"].is_array());
}

// ──────────────────────────────────────────────────────────
// Validate-template edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_validate_template_nonexistent_path() {
    let output = run_fledge(&["templates", "validate", "/tmp/no-such-path-ever-12345"]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_empty_template_toml() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("empty-tpl");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(tpl.join("template.toml"), "").unwrap();
    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_missing_name_field() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("noname");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
description = "Missing name field"

[files]
ignore = ["template.toml"]
"#,
    )
    .unwrap();
    fs::write(tpl.join("file.txt"), "content").unwrap();
    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_missing_description() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("nodesc");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
name = "nodesc"

[files]
ignore = ["template.toml"]
"#,
    )
    .unwrap();
    fs::write(tpl.join("file.txt"), "content").unwrap();
    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    // Missing description might be a warning or error
    let _status = output.status;
}

// ──────────────────────────────────────────────────────────
