mod common;
use common::*;

use std::fs;
use tempfile::TempDir;

// Run (task runner) commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_run_no_fledge_toml_generic_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("Could not detect project type"),
        "expected detection failure error, got: {stderr}"
    );
}

#[test]
fn cli_run_auto_detect_rust() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Auto-detected"),
        "expected auto-detect banner, got: {stdout}"
    );
    assert!(stdout.contains("build"), "expected build task in output");
    assert!(stdout.contains("test"), "expected test task in output");
}

#[test]
fn cli_run_auto_detect_node() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"scripts":{"build":"tsc","test":"jest","dev":"vite"}}"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Auto-detected"),
        "expected auto-detect banner"
    );
    assert!(stdout.contains("build"), "expected build task");
    assert!(stdout.contains("dev"), "expected dev task");
}

#[test]
fn cli_run_auto_detect_bun() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"scripts":{"build":"tsc","test":"bun test"}}"#,
    )
    .unwrap();
    std::fs::write(tmp.path().join("bun.lockb"), "").unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("bun"),
        "expected bun runner in output, got: {stdout}"
    );
}

#[test]
fn cli_run_fledge_toml_overrides_auto_detect() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\ncustom = \"echo hello\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("Auto-detected"),
        "should not show auto-detect when fledge.toml exists"
    );
    assert!(
        stdout.contains("custom"),
        "expected custom task from fledge.toml"
    );
}

#[test]
fn cli_run_init_creates_fledge_toml() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init"]);
    assert!(output.status.success());
    assert!(tmp.path().join("fledge.toml").exists());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("[tasks]"));
}

#[test]
fn cli_run_init_detects_rust() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init"]);
    assert!(output.status.success());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("cargo"));
    assert!(content.contains("rust"));
}

#[test]
fn cli_run_init_detects_node() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("package.json"), "{}").unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init"]);
    assert!(output.status.success());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("npm") || content.contains("node"));
}

#[test]
fn cli_run_init_detects_go() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("go.mod"), "module example.com/test\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init"]);
    assert!(output.status.success());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("go"));
}

#[test]
fn cli_run_init_detects_python() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("pyproject.toml"), "[tool]\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init"]);
    assert!(output.status.success());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("python"));
}

#[test]
fn cli_run_init_wont_overwrite() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("fledge.toml"), "[tasks]\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("already exists"));
}

#[test]
fn cli_run_list_shows_tasks() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\ntest = \"echo test\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("build"));
    assert!(stdout.contains("test"));
}

#[test]
fn cli_run_unknown_task_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "nonexistent"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unknown task"));
}

#[test]
fn cli_run_task_executes() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nhello = \"echo hello-from-fledge\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "hello"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello-from-fledge"));
}

#[test]
fn cli_run_empty_tasks_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("fledge.toml"), "[tasks]\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No tasks defined"));
}

#[test]
fn cli_run_task_with_deps() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
prep = "echo PREP"

[tasks.build]
cmd = "echo BUILD"
deps = ["prep"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "build"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PREP"));
    assert!(stdout.contains("BUILD"));
}

#[test]
fn cli_run_failing_task_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nfail = \"exit 1\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "fail"]);
    assert!(!output.status.success());
}

// ──────────────────────────────────────────────────────────
