mod common;
use common::*;

use std::fs;
use tempfile::TempDir;

// Lane commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_lane_no_fledge_toml_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["lane"]);
    assert!(!output.status.success());
}

#[test]
fn cli_lane_list_shows_lanes() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
fmt = "echo fmt"
lint = "echo lint"
test = "echo test"

[lanes.ci]
description = "CI pipeline"
steps = ["fmt", "lint", "test"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ci"));
}

#[test]
fn cli_lane_dry_run_does_not_execute() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo BUILT"

[lanes.ci]
description = "CI"
steps = ["build"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "ci", "--dry-run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("BUILT"));
}

#[test]
fn cli_lane_executes_steps() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
step1 = "echo STEP1"
step2 = "echo STEP2"

[lanes.pipeline]
steps = ["step1", "step2"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "pipeline"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("STEP1"));
    assert!(stdout.contains("STEP2"));
}

#[test]
fn cli_lane_unknown_lane_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n[lanes.ci]\nsteps = [\"build\"]\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "nonexistent"]);
    assert!(!output.status.success());
}

#[test]
fn cli_lane_init_adds_default_lanes() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "init"]);
    assert!(output.status.success());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("[lanes"));
}

#[test]
fn cli_lane_json_output() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo build"

[lanes.ci]
description = "CI"
steps = ["build"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_object() || parsed.is_array());
}

// ──────────────────────────────────────────────────────────
// JSON envelope tests (issue #271 — tier B)
// ──────────────────────────────────────────────────────────

#[test]
fn cli_lane_init_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "init", "--json"]);
    assert!(
        output.status.success(),
        "lane init --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("init"));
    assert!(parsed["project_type"].is_string());
    assert!(parsed["lanes_added"].is_array());
}

#[test]
fn cli_lane_import_json_error_path_returns_nonzero() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n",
    )
    .unwrap();
    let output = run_fledge_in(
        tmp.path(),
        &[
            "lane",
            "import",
            "nonexistent/repo-does-not-exist",
            "--json",
        ],
    );
    assert!(
        !output.status.success(),
        "lane import of nonexistent repo must exit nonzero even with --json"
    );
}

#[test]
fn cli_lane_publish_json_error_path_returns_nonzero() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "publish", "--json"]);
    assert!(
        !output.status.success(),
        "lane publish in empty dir must exit nonzero even with --json"
    );
}

#[test]
fn cli_lane_create_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(
        tmp.path(),
        &[
            "lane",
            "create",
            "my-lanes",
            "--output",
            tmp.path().to_str().unwrap(),
            "--yes",
            "--json",
        ],
    );
    assert!(
        output.status.success(),
        "lane create --json failed: {}",
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

// ──────────────────────────────────────────────────────────
