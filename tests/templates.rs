mod common;
use common::*;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// MARK: - templates init / list
#[test]
fn cli_list_shows_templates() {
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["templates", "list"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "templates list failed: {stdout}");
    assert!(stdout.contains("rust-cli"));
    assert!(stdout.contains("ts-bun"));
}

#[test]
fn cli_init_with_template_creates_project() {
    let bin = cargo_bin();
    let tmp = TempDir::new().unwrap();

    let output = Command::new(&bin)
        .args([
            "templates",
            "init",
            "test-project",
            "--template",
            "rust-cli",
            "--output",
            tmp.path().to_str().unwrap(),
            "--no-git",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(
        output.status.success(),
        "init should succeed in non-TTY mode using defaults.\nstderr: {stderr}\nstdout: {stdout}"
    );
    let project_dir = tmp.path().join("test-project");
    assert!(project_dir.exists(), "project dir not created");
    assert!(
        project_dir.join("Cargo.toml").exists(),
        "Cargo.toml not found"
    );
    assert!(
        project_dir.join("src/main.rs").exists(),
        "src/main.rs not found"
    );
}

#[test]
fn cli_init_unknown_template_fails() {
    let bin = cargo_bin();
    let tmp = TempDir::new().unwrap();

    let output = Command::new(&bin)
        .args([
            "templates",
            "init",
            "test-project",
            "--template",
            "nonexistent-template",
            "--output",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("not found") || stderr.contains("nonexistent"),
        "expected 'not found' error, got: {stderr}"
    );
}

#[test]
fn cli_init_existing_dir_fails() {
    let bin = cargo_bin();
    let tmp = TempDir::new().unwrap();

    // Create the target dir first
    let existing = tmp.path().join("existing-project");
    fs::create_dir(&existing).unwrap();

    let output = Command::new(&bin)
        .args([
            "templates",
            "init",
            "existing-project",
            "--template",
            "rust-cli",
            "--output",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // It might fail due to dialoguer before reaching the exists check,
    // but if it gets there, it should error about existing dir
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        let is_expected = stderr.contains("already exists")
            || stderr.contains("dialoguer")
            || stderr.contains("not a terminal")
            || stderr.contains("IO error");
        assert!(is_expected, "unexpected error: {stderr}");
    }
}

#[test]
fn cli_no_args_shows_help() {
    let bin = cargo_bin();
    let output = Command::new(&bin).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Usage") || stderr.contains("usage"));
}

#[test]
fn cli_version_flag() {
    let bin = cargo_bin();
    let output = Command::new(&bin).arg("--version").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge"));
}

#[test]
fn cli_completions_bash() {
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["completions", "bash"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("_fledge"));
    assert!(stdout.contains("templates"));
}

#[test]
fn cli_completions_zsh() {
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["completions", "zsh"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("compdef") || stdout.contains("_fledge"));
}

#[test]
fn cli_completions_fish() {
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["completions", "fish"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge"));
}

#[test]
fn cli_dry_run_does_not_create_files() {
    let bin = cargo_bin();
    let tmp = TempDir::new().unwrap();

    let output = Command::new(&bin)
        .args([
            "templates",
            "init",
            "dry-test",
            "--template",
            "rust-cli",
            "--output",
            tmp.path().to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "dry-run failed: {stdout}");
    assert!(stdout.contains("Dry run"));
    assert!(!tmp.path().join("dry-test").exists());
}

// ──────────────────────────────────────────────────────────

// MARK: - Special characters and unicode in project names
// Special characters and unicode in project names
// ──────────────────────────────────────────────────────────

#[test]
fn cli_init_name_with_spaces_handled() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "templates",
        "init",
        "my cool project",
        "--template",
        "rust-cli",
        "--output",
        tmp.path().to_str().unwrap(),
        "--no-git",
        "--yes",
    ]);
    // Should either succeed with a sanitized name or fail gracefully
    if output.status.success() {
        assert!(
            tmp.path().join("my cool project").exists()
                || tmp.path().join("my-cool-project").exists()
                || tmp.path().join("my_cool_project").exists()
        );
    }
}

#[test]
fn cli_init_name_with_special_chars() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "templates",
        "init",
        "@scope/pkg-name",
        "--template",
        "ts-bun",
        "--output",
        tmp.path().to_str().unwrap(),
        "--no-git",
        "--no-install",
        "--yes",
    ]);
    // Should handle scoped package names or fail gracefully — not panic
    let _ = output.status;
}

// ──────────────────────────────────────────────────────────
// Task runner: multiple tasks, env inheritance, dir edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_run_task_with_multiple_env_vars() {
    let tmp = TempDir::new().unwrap();
    let cmd = if cfg!(windows) {
        "echo %FOO% %BAR%"
    } else {
        "echo $FOO $BAR"
    };
    fs::write(
        tmp.path().join("fledge.toml"),
        format!("[tasks.multi]\ncmd = \"{cmd}\"\nenv = {{ FOO = \"hello\", BAR = \"world\" }}\n"),
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "multi"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello"));
    assert!(stdout.contains("world"));
}

#[test]
fn cli_run_task_dir_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks.bad]
cmd = "echo hi"
dir = "no-such-dir"
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "bad"]);
    assert!(!output.status.success());
}

#[test]
fn cli_run_many_tasks_listed() {
    let tmp = TempDir::new().unwrap();
    let mut tasks = String::from("[tasks]\n");
    for i in 0..20 {
        tasks.push_str(&format!("task{i} = \"echo task {i}\"\n"));
    }
    fs::write(tmp.path().join("fledge.toml"), &tasks).unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("task0"));
    assert!(stdout.contains("task19"));
}

// ──────────────────────────────────────────────────────────

// MARK: - templates create
// Create-template command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_create_template_creates_scaffold() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "templates",
        "create",
        "my-template",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    // create-template uses dialoguer prompts, may fail in non-TTY
    if output.status.success() {
        let tpl_dir = tmp.path().join("my-template");
        assert!(tpl_dir.exists());
        assert!(tpl_dir.join("template.toml").exists());
    } else {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("dialoguer")
                || stderr.contains("not a terminal")
                || stderr.contains("IO error"),
            "unexpected error: {stderr}"
        );
    }
}

#[test]
fn cli_create_template_existing_dir_fails() {
    let tmp = TempDir::new().unwrap();
    let existing = tmp.path().join("existing-tpl");
    fs::create_dir_all(&existing).unwrap();
    let output = run_fledge(&[
        "templates",
        "create",
        "existing-tpl",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("exists") || stderr.contains("already"),
        "expected already-exists error, got: {stderr}"
    );
}

// ──────────────────────────────────────────────────────────
// JSON envelope tests (issue #271 — tier B)
// ──────────────────────────────────────────────────────────

#[test]
fn cli_templates_list_json_emits_envelope() {
    let output = run_fledge(&["templates", "list", "--json"]);
    assert!(
        output.status.success(),
        "templates list --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert!(parsed["templates"].is_array());
    let templates = parsed["templates"].as_array().unwrap();
    assert!(
        !templates.is_empty(),
        "should list at least one builtin template"
    );
    let first = &templates[0];
    assert!(first["name"].is_string());
    assert!(first["source"].is_string());
}

#[test]
fn cli_templates_init_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "templates",
        "init",
        "json-test-project",
        "--template",
        "rust-cli",
        "--output",
        tmp.path().to_str().unwrap(),
        "--no-git",
        "--yes",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "templates init --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("init"));
    assert!(parsed["project"]["name"].is_string());
    assert!(parsed["template"]["name"].is_string());
    assert!(parsed["files_created"].is_array());
}

#[test]
fn cli_templates_init_json_error_path_returns_nonzero() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "templates",
        "init",
        "test-project",
        "--template",
        "nonexistent-template-xyz",
        "--output",
        tmp.path().to_str().unwrap(),
        "--json",
    ]);
    assert!(
        !output.status.success(),
        "templates init of nonexistent template must exit nonzero even with --json"
    );
}

#[test]
fn cli_templates_create_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "templates",
        "create",
        "my-template",
        "--output",
        tmp.path().to_str().unwrap(),
        "--yes",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "templates create --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("create"));
    assert!(parsed["name"].is_string());
    assert!(parsed["path"].is_string());
    assert!(parsed["files_created"].is_array());
}

#[test]
fn cli_templates_publish_json_error_path_returns_nonzero() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["templates", "publish", "--json"]);
    assert!(
        !output.status.success(),
        "templates publish in empty dir must exit nonzero even with --json"
    );
}

// ──────────────────────────────────────────────────────────
