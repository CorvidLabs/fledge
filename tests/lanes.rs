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
    // Post-tier-C envelope: {schema_version: 1, lanes: [...]}
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert!(parsed["lanes"].is_array());
}

#[test]
fn cli_lane_run_json_stdout_is_clean() {
    // Regression guard: in JSON mode `lanes run --json` must emit only the
    // envelope on stdout. Discovered during tier-C testing — fledge's own
    // progress prose ("▶️ Running task: ...") and the spawned task's stdout
    // ("BUILT") used to interleave with the JSON, making `--json | jq`
    // unparseable. The fix threads a `quiet` flag into the executor chain.
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo BUILT_OUTPUT"
test = "echo TEST_OUTPUT"

[lanes.ci]
description = "CI"
steps = ["build", "test"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "ci", "--json"]);
    assert!(
        output.status.success(),
        "lane run --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Stdout must parse as a single JSON value — no prose, no task output.
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not a single JSON object — prose or task output leaked through.\n\
             error: {e}\n\
             stdout was:\n{stdout}"
        )
    });
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["lane"].as_str(), Some("ci"));
    assert_eq!(parsed["success"].as_bool(), Some(true));
    // Task output must NOT have leaked into stdout.
    assert!(
        !stdout.contains("BUILT_OUTPUT"),
        "task stdout leaked into agent stdout in --json mode"
    );
    assert!(
        !stdout.contains("TEST_OUTPUT"),
        "task stdout leaked into agent stdout in --json mode"
    );
    // Fledge's own progress prose must NOT have leaked either.
    assert!(
        !stdout.contains("▶️"),
        "fledge progress prose leaked into agent stdout in --json mode"
    );
}

// ──────────────────────────────────────────────────────────
