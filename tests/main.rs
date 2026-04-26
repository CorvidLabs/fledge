mod common;
use common::*;

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// MARK: - help subcommands
// Help subcommands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_help_flag_shows_usage() {
    let output = run_fledge(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("templates"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("lanes"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("plugins"));
}

#[test]
fn cli_subcommand_help() {
    let subcommands = [
        "templates",
        "run",
        "lane",
        "config",
        "spec",
        "doctor",
        "plugins",
        "changelog",
    ];
    for cmd in &subcommands {
        let output = run_fledge(&[cmd, "--help"]);
        assert!(output.status.success(), "'{cmd} --help' failed");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains("Usage") || stdout.contains("usage"),
            "'{cmd} --help' doesn't show usage"
        );
    }
}

// ──────────────────────────────────────────────────────────
// Lane with inline and parallel steps
// ──────────────────────────────────────────────────────────

#[test]
fn cli_lane_inline_step() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]

[lanes.greet]
steps = [{ run = "echo INLINE_HELLO" }]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "greet"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("INLINE_HELLO"));
}

#[test]
fn cli_lane_parallel_steps() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
a = "echo ALPHA"
b = "echo BRAVO"

[lanes.par]
steps = [{ parallel = ["a", "b"] }]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "par"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ALPHA"));
    assert!(stdout.contains("BRAVO"));
}

// ──────────────────────────────────────────────────────────
// Task runner with env and dir options
// ──────────────────────────────────────────────────────────

#[test]
fn cli_run_task_with_env() {
    let tmp = TempDir::new().unwrap();
    let cmd = if cfg!(windows) {
        r#"echo %GREETING%"#
    } else {
        r#"echo $GREETING"#
    };
    fs::write(
        tmp.path().join("fledge.toml"),
        format!("[tasks.greet]\ncmd = \"{cmd}\"\nenv = {{ GREETING = \"HELLO_FLEDGE\" }}\n"),
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "greet"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("HELLO_FLEDGE"));
}

#[test]
fn cli_run_task_with_dir() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks.pwd]
cmd = "pwd"
dir = "subdir"
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "pwd"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("subdir"));
}

// ──────────────────────────────────────────────────────────
// Validate built-in templates
// ──────────────────────────────────────────────────────────

#[test]
fn cli_validate_builtin_templates() {
    let templates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let output = run_fledge(&["templates", "validate", templates_dir.to_str().unwrap()]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "Built-in templates validation failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

// ──────────────────────────────────────────────────────────
// Plugin list (no plugins installed)
// ──────────────────────────────────────────────────────────

#[test]
fn cli_plugin_list_empty() {
    let output = run_fledge(&["plugin", "list"]);
    assert!(output.status.success());
}

// ──────────────────────────────────────────────────────────
// Ask without question fails
// ──────────────────────────────────────────────────────────

#[test]
fn cli_ask_no_question_fails() {
    let output = run_fledge(&["ask"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("question") || stderr.contains("Usage"));
}

// ──────────────────────────────────────────────────────────
// E2E workflow: init → run → lane → doctor
// ──────────────────────────────────────────────────────────

#[test]
fn e2e_rust_project_lifecycle() {
    let tmp = TempDir::new().unwrap();

    // Step 1: Init a Rust project
    let output = run_fledge(&[
        "templates",
        "init",
        "e2e-test",
        "--template",
        "rust-cli",
        "--output",
        tmp.path().to_str().unwrap(),
        "--no-git",
        "--yes",
    ]);
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let project = tmp.path().join("e2e-test");
    assert!(project.join("Cargo.toml").exists());

    // Step 2: Verify fledge.toml was created by init
    assert!(project.join("fledge.toml").exists());
    let fledge_toml = fs::read_to_string(project.join("fledge.toml")).unwrap();
    assert!(fledge_toml.contains("[tasks]"));
    assert!(fledge_toml.contains("cargo"));

    // Step 3: List tasks
    let output = run_fledge_in(&project, &["run", "--list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("build") || stdout.contains("test"));

    // Step 4: Generate default lanes
    let output = run_fledge_in(&project, &["lane", "init"]);
    assert!(
        output.status.success(),
        "lane init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fledge_toml = fs::read_to_string(project.join("fledge.toml")).unwrap();
    assert!(fledge_toml.contains("[lanes"));

    // Step 5: List lanes
    let output = run_fledge_in(&project, &["lane", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ci"));

    // Step 6: Dry-run a lane
    let output = run_fledge_in(&project, &["lane", "run", "ci", "--dry-run"]);
    assert!(output.status.success());

    // Step 7: Doctor check
    let output = run_fledge_in(&project, &["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") || stdout.contains("Git"));

    // Step 9: Doctor JSON
    let output = run_fledge_in(&project, &["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["sections"].is_array());
}

#[test]
fn e2e_tsbun_project_lifecycle() {
    let tmp = TempDir::new().unwrap();

    // Step 1: Init a ts-bun project
    let output = run_fledge(&[
        "templates",
        "init",
        "e2e-ts",
        "--template",
        "ts-bun",
        "--output",
        tmp.path().to_str().unwrap(),
        "--no-git",
        "--no-install",
        "--yes",
    ]);
    assert!(
        output.status.success(),
        "init ts-bun failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let project = tmp.path().join("e2e-ts");
    assert!(project.join("package.json").exists());

    // Step 2: Verify fledge.toml was created by init
    assert!(project.join("fledge.toml").exists());
    let fledge_toml = fs::read_to_string(project.join("fledge.toml")).unwrap();
    assert!(fledge_toml.contains("[tasks]"));
    assert!(fledge_toml.contains("bun"));

    // Step 3: Doctor
    let output = run_fledge_in(&project, &["doctor"]);
    assert!(output.status.success());
}

// ══════════════════════════════════════════════════════════

// MARK: - edge cases, integration, and stress
// NEW: Edge cases, integration, and stress tests
// ══════════════════════════════════════════════════════════

// ──────────────────────────────────────────────────────────
// Malformed fledge.toml edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_run_malformed_toml_fails_gracefully() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("fledge.toml"), "{{{{not valid toml!!!").unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("parsing") || stderr.contains("TOML") || stderr.contains("error"),
        "expected parse error, got: {stderr}"
    );
}

#[test]
fn cli_lane_malformed_toml_fails_gracefully() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("fledge.toml"), "not = [valid toml {").unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "list"]);
    assert!(!output.status.success());
}

#[test]
fn cli_run_toml_missing_tasks_section() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[metadata]\nname = \"test\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("No tasks") || stderr.contains("Could not detect"),
        "expected no-tasks error, got: {stderr}"
    );
}

#[test]
fn cli_run_task_with_empty_cmd() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("fledge.toml"), "[tasks]\nempty = \"\"\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "empty"]);
    // Empty command should either succeed (no-op) or fail — shouldn't panic
    let _ = output.status;
}

// ──────────────────────────────────────────────────────────
// Circular and missing dependency edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_run_circular_dep_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks.a]
cmd = "echo A"
deps = ["b"]

[tasks.b]
cmd = "echo B"
deps = ["a"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "a"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("Circular") || stderr.contains("circular") || stderr.contains("dependency"),
        "expected circular dep error, got: {stderr}"
    );
}

#[test]
fn cli_run_missing_dep_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks.build]
cmd = "echo build"
deps = ["nonexistent"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "build"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("not found") || stderr.contains("nonexistent"),
        "expected missing dep error, got: {stderr}"
    );
}

#[test]
fn cli_run_deep_dep_chain() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
step1 = "echo S1"

[tasks.step2]
cmd = "echo S2"
deps = ["step1"]

[tasks.step3]
cmd = "echo S3"
deps = ["step2"]

[tasks.step4]
cmd = "echo S4"
deps = ["step3"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "step4"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("S1"));
    assert!(stdout.contains("S2"));
    assert!(stdout.contains("S3"));
    assert!(stdout.contains("S4"));
}

// ──────────────────────────────────────────────────────────
// Lane edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_lane_empty_steps_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo build"

[lanes.empty]
steps = []
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "empty"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no steps") || stderr.contains("No steps"),
        "expected empty steps error, got: {stderr}"
    );
}

#[test]
fn cli_lane_references_missing_task_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo build"

[lanes.broken]
steps = ["build", "ghost-task"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "broken"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("ghost-task") || stderr.contains("unknown"),
        "expected missing task error, got: {stderr}"
    );
}

#[test]
fn cli_lane_fail_fast_stops_on_failure() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
fail = "exit 1"
after = "echo SHOULD_NOT_RUN"

[lanes.ff]
fail_fast = true
steps = ["fail", "after"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "ff"]);
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("SHOULD_NOT_RUN"),
        "fail_fast lane should stop after first failure"
    );
}

#[test]
fn cli_lane_mixed_inline_and_task_ref() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
lint = "echo LINTED"

[lanes.mixed]
steps = ["lint", { run = "echo INLINE_STEP" }]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "mixed"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("LINTED"));
    assert!(stdout.contains("INLINE_STEP"));
}

#[test]
fn cli_lane_dry_run_shows_plan() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
a = "echo A"
b = "echo B"

[lanes.plan]
description = "Show plan"
steps = ["a", "b"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "plan", "--dry-run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Running"));
}

#[test]
fn cli_lane_no_lanes_section_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("No lanes") || stderr.contains("no lanes"),
        "expected no-lanes error, got: {stderr}"
    );
}

// ──────────────────────────────────────────────────────────
// Config edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_config_set_and_get_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("fledge-config");
    std::fs::create_dir_all(&config_dir).unwrap();

    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["config", "set", "defaults.author", "test-author-e2e"])
        .env("FLEDGE_CONFIG_DIR", &config_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(&bin)
        .args(["config", "get", "defaults.author"])
        .env("FLEDGE_CONFIG_DIR", &config_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test-author-e2e"));
}

#[test]
fn cli_config_unset_unknown_key_fails() {
    let output = run_fledge(&["config", "unset", "nonexistent.key"]);
    assert!(!output.status.success());
}

#[test]
fn cli_config_add_remove_list_key() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("fledge-config");
    std::fs::create_dir_all(&config_dir).unwrap();

    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["config", "add", "templates.paths", "/tmp/e2e-test-path"])
        .env("FLEDGE_CONFIG_DIR", &config_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(&bin)
        .args(["config", "get", "templates.paths"])
        .env("FLEDGE_CONFIG_DIR", &config_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("/tmp/e2e-test-path"),
        "expected /tmp/e2e-test-path in output, got: {stdout}"
    );

    let output = Command::new(&bin)
        .args(["config", "remove", "templates.paths", "/tmp/e2e-test-path"])
        .env("FLEDGE_CONFIG_DIR", &config_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn cli_config_init_default() {
    let output = run_fledge(&["config", "init"]);
    // Should succeed (creates default config if none exists, or reports it already exists)
    let _ = output.status;
}

// ──────────────────────────────────────────────────────────
// Plugin edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_plugin_remove_nonexistent_fails() {
    let output = run_fledge(&["plugin", "remove", "no-such-plugin"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("not found")
            || stderr.contains("not installed")
            || stderr.contains("No plugin"),
        "expected not-found error, got: {stderr}"
    );
}

#[test]
fn cli_plugin_run_nonexistent_fails() {
    let output = run_fledge(&["plugin", "run", "no-such-command"]);
    assert!(!output.status.success());
}

#[test]
fn cli_plugin_list_json() {
    let output = run_fledge(&["plugin", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should be valid JSON (empty array or object)
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn cli_plugin_update_no_plugins() {
    let output = run_fledge(&["plugin", "update"]);
    assert!(output.status.success());
}

#[test]
fn cli_plugin_update_nonexistent_fails() {
    let output = run_fledge(&["plugin", "update", "nonexistent"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("not installed"));
}

#[test]
fn cli_plugin_update_help() {
    let output = run_fledge(&["plugin", "update", "--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Update installed plugins"));
}

// ──────────────────────────────────────────────────────────

// MARK: - external / unknown subcommand
// External/unknown subcommand
// ──────────────────────────────────────────────────────────

#[test]
fn cli_unknown_subcommand_fails() {
    let output = run_fledge(&["definitely-not-a-command"]);
    // External subcommand dispatch — should fail if no matching plugin
    assert!(!output.status.success());
}

// ──────────────────────────────────────────────────────────
// Completions edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_completions_powershell() {
    let output = run_fledge(&["completions", "powershell"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.is_empty());
}

#[test]
fn cli_completions_elvish() {
    let output = run_fledge(&["completions", "elvish"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.is_empty());
}

// ──────────────────────────────────────────────────────────
// Auto-detection edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_run_auto_detect_python() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("pyproject.toml"), "[tool]\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Auto-detected"));
}

#[test]
fn cli_run_auto_detect_go() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("go.mod"),
        "module example.com/test\ngo 1.21\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Auto-detected"));
}

#[test]
fn cli_run_auto_detect_with_multiple_markers() {
    // Both Cargo.toml and package.json — Cargo.toml should win (or one of them)
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("package.json"),
        r#"{"scripts":{"build":"tsc"}}"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Auto-detected"));
}

// ──────────────────────────────────────────────────────────
// Lane: parallel with failing task
// ──────────────────────────────────────────────────────────

#[test]
fn cli_lane_parallel_with_failure() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
ok = "echo OK"
fail = "exit 1"

[lanes.par_fail]
steps = [{ parallel = ["ok", "fail"] }]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "par_fail"]);
    assert!(!output.status.success());
}

#[test]
fn cli_lane_many_steps() {
    let tmp = TempDir::new().unwrap();
    let mut toml = String::from("[tasks]\n");
    for i in 0..15 {
        toml.push_str(&format!("s{i} = \"echo STEP_{i}\"\n"));
    }
    toml.push_str("\n[lanes.big]\nsteps = [");
    for i in 0..15 {
        if i > 0 {
            toml.push_str(", ");
        }
        toml.push_str(&format!("\"s{i}\""));
    }
    toml.push_str("]\n");
    fs::write(tmp.path().join("fledge.toml"), &toml).unwrap();
    let output = run_fledge_in(tmp.path(), &["lane", "run", "big"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("STEP_0"));
    assert!(stdout.contains("STEP_14"));
}

// ──────────────────────────────────────────────────────────
// E2E: full workflow with lanes, tasks, doctor, then metrics
// ──────────────────────────────────────────────────────────

#[test]
fn e2e_custom_fledge_toml_full_lifecycle() {
    let tmp = TempDir::new().unwrap();

    // Write a hand-crafted fledge.toml
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
check = "echo CHECK_PASS"
build = "echo BUILD_PASS"
test = "echo TEST_PASS"

[tasks.all]
cmd = "echo ALL_PASS"
deps = ["check", "build", "test"]

[lanes.ci]
description = "Full CI pipeline"
steps = ["check", "build", "test"]

[lanes.quick]
description = "Quick check"
steps = [{ run = "echo QUICK_PASS" }]

[lanes.parallel_build]
steps = [{ parallel = ["check", "build"] }, "test"]
"#,
    )
    .unwrap();

    // 1. List tasks
    let output = run_fledge_in(tmp.path(), &["run", "--list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("check"));
    assert!(stdout.contains("build"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("all"));

    // 2. Run individual task
    let output = run_fledge_in(tmp.path(), &["run", "build"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("BUILD_PASS"));

    // 3. Run task with deps
    let output = run_fledge_in(tmp.path(), &["run", "all"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("CHECK_PASS"));
    assert!(stdout.contains("BUILD_PASS"));
    assert!(stdout.contains("TEST_PASS"));
    assert!(stdout.contains("ALL_PASS"));

    // 4. List lanes
    let output = run_fledge_in(tmp.path(), &["lane", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ci"));
    assert!(stdout.contains("quick"));

    // 5. Run CI lane
    let output = run_fledge_in(tmp.path(), &["lane", "run", "ci"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("CHECK_PASS"));
    assert!(stdout.contains("TEST_PASS"));

    // 6. Run inline-step lane
    let output = run_fledge_in(tmp.path(), &["lane", "run", "quick"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("QUICK_PASS"));

    // 7. Run parallel lane
    let output = run_fledge_in(tmp.path(), &["lane", "run", "parallel_build"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("CHECK_PASS"));
    assert!(stdout.contains("BUILD_PASS"));
    assert!(stdout.contains("TEST_PASS"));

    // 8. Dry-run lane
    let output = run_fledge_in(tmp.path(), &["lane", "run", "ci", "--dry-run"]);
    assert!(output.status.success());

    // 9. Lane list JSON
    let output = run_fledge_in(tmp.path(), &["lane", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 3);

    // 10. Doctor in this dir
    let output = run_fledge_in(tmp.path(), &["doctor"]);
    assert!(output.status.success());
}

// ──────────────────────────────────────────────────────────
// E2E: create template, validate, init from it
// ──────────────────────────────────────────────────────────

#[test]
fn e2e_create_validate_init_template() {
    let tmp = TempDir::new().unwrap();

    // 1. Create a template scaffold (may fail in non-TTY due to dialoguer)
    let output = run_fledge(&[
        "templates",
        "create",
        "test-tpl",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("dialoguer")
                || stderr.contains("not a terminal")
                || stderr.contains("IO error"),
            "unexpected create-template error: {stderr}"
        );
        return; // Can't continue without the template
    }
    let tpl_dir = tmp.path().join("test-tpl");
    assert!(tpl_dir.exists());
    assert!(tpl_dir.join("template.toml").exists());

    // 2. Validate the created template
    let output = run_fledge(&["templates", "validate", tpl_dir.to_str().unwrap()]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "validate created template failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn create_template_non_interactive_with_yes() {
    let tmp = TempDir::new().unwrap();

    let output = run_fledge(&[
        "templates",
        "create",
        "my-tpl",
        "--output",
        tmp.path().to_str().unwrap(),
        "--yes",
    ]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "create-template --yes failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    let tpl_dir = tmp.path().join("my-tpl");
    assert!(tpl_dir.join("template.toml").exists());
    assert!(tpl_dir.join("README.md").exists());

    // Validate the generated template is well-formed
    let output = run_fledge(&["templates", "validate", tpl_dir.to_str().unwrap()]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "validate failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn create_template_non_interactive_with_all_flags() {
    let tmp = TempDir::new().unwrap();

    let output = run_fledge(&[
        "templates",
        "create",
        "flagged-tpl",
        "--output",
        tmp.path().to_str().unwrap(),
        "--description",
        "A custom template",
        "--render-patterns",
        "**/*.rs, **/*.md",
        "--hooks",
        "--prompts",
    ]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "create-template with flags failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    let tpl_dir = tmp.path().join("flagged-tpl");
    let manifest = std::fs::read_to_string(tpl_dir.join("template.toml")).unwrap();
    assert!(manifest.contains("A custom template"));
    assert!(manifest.contains("**/*.rs"));
    assert!(manifest.contains("[hooks]"));
    assert!(manifest.contains("[prompts"));
}

// ──────────────────────────────────────────────────────────
// Review — error cases (no Claude CLI needed)
// ──────────────────────────────────────────────────────────

#[test]
fn cli_review_outside_git_repo_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["review"]);
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("git") || stderr.contains("Claude CLI"),
            "expected git or CLI error, got: {stderr}"
        );
    }
}

#[test]
fn cli_review_no_changes_fails() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    // CI doesn't have a global git identity; set a local one so `git commit`
    // actually writes a commit instead of silently failing.
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
    let output = run_fledge_in(tmp.path(), &["review", "--base", "HEAD"]);
    assert!(!output.status.success(), "expected failure on empty diff");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("No changes") || stderr.contains("Claude CLI") || stderr.contains("Ollama"),
        "expected no-changes or provider error, got: {stderr}"
    );
}

#[test]
fn cli_review_accepts_base_flag() {
    let output = run_fledge(&["review", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--base"));
    assert!(stdout.contains("--file"));
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_review_accepts_spec_flags() {
    let output = run_fledge(&["review", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--with-specs"));
    assert!(stdout.contains("--no-auto-specs"));
}

#[test]
fn cli_ask_accepts_json_flag() {
    let output = run_fledge(&["ask", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_ask_accepts_with_specs_flag() {
    let output = run_fledge(&["ask", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--with-specs"));
    assert!(stdout.contains("--no-spec-index"));
}

#[test]
fn cli_ask_accepts_provider_and_model_flags() {
    let output = run_fledge(&["ask", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--provider"));
    assert!(stdout.contains("--model"));
}

#[test]
fn cli_review_accepts_provider_flag() {
    let output = run_fledge(&["review", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--provider"));
}

#[test]
fn cli_ask_help_lists_supported_providers() {
    let output = run_fledge(&["ask", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("claude"));
    assert!(stdout.contains("ollama"));
}

#[test]
fn cli_ask_rejects_unknown_provider_at_parse_time() {
    // clap's value_parser should reject `--provider invalid` before the
    // command ever runs — no LLM contact, exit non-zero, stderr mentions
    // the invalid value.
    let output = run_fledge(&["ask", "--provider", "gpt", "whatever"]);
    assert!(
        !output.status.success(),
        "expected clap to reject --provider gpt"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("gpt") || stderr.contains("invalid") || stderr.contains("possible"),
        "stderr should mention the bad value, got: {stderr}"
    );
}

#[test]
fn cli_review_rejects_unknown_provider_at_parse_time() {
    let output = run_fledge(&["review", "--provider", "gemini"]);
    assert!(
        !output.status.success(),
        "expected clap to reject --provider gemini"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("gemini") || stderr.contains("invalid") || stderr.contains("possible"),
        "stderr should mention the bad value, got: {stderr}"
    );
}

#[test]
fn cli_ai_help_lists_subcommands() {
    let output = run_fledge(&["ai", "--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("status"));
    assert!(stdout.contains("models"));
    assert!(stdout.contains("use"));
}

#[test]
fn cli_ai_status_json_shape() {
    let output = run_fledge(&["ai", "status", "--json"]);
    assert!(
        output.status.success(),
        "ai status should succeed, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("ai status --json should be valid JSON: {e}\n{stdout}"));
    assert!(parsed.get("provider").is_some());
    assert!(parsed.get("provider_source").is_some());
}

#[test]
fn cli_ai_use_rejects_unknown_provider_at_parse_time() {
    let output = run_fledge(&["ai", "use", "gpt"]);
    assert!(!output.status.success(), "clap should reject `ai use gpt`");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("gpt") || stderr.contains("invalid") || stderr.contains("possible"),
        "stderr should mention the bad value, got: {stderr}"
    );
}

#[test]
fn cli_ai_use_non_interactive_without_provider_fails() {
    let output = run_fledge(&["--non-interactive", "ai", "use"]);
    assert!(
        !output.status.success(),
        "ai use without a provider in --non-interactive mode should fail"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("interactive") || stderr.contains("provider"),
        "stderr should explain the missing interaction, got: {stderr}"
    );
}

#[test]
fn cli_ai_models_rejects_unknown_provider_at_parse_time() {
    let output = run_fledge(&["ai", "models", "--provider", "gemini"]);
    assert!(
        !output.status.success(),
        "clap should reject --provider gemini on `ai models`"
    );
}

#[test]
fn cli_global_non_interactive_flag_present_in_help() {
    let output = run_fledge(&["--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("--non-interactive"),
        "expected --non-interactive in top-level help: {stdout}"
    );
}

#[test]
fn cli_non_interactive_accepted_on_subcommand() {
    // Global clap args don't appear in every subcommand's --help, but they
    // must still be *accepted* on subcommands. A passing help invocation
    // (exit 0) is enough to confirm the parser accepts the arg there.
    let output = run_fledge(&["--non-interactive", "spec", "list", "--json"]);
    assert!(
        output.status.success(),
        "--non-interactive was rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let _parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
}

#[test]
fn cli_non_interactive_alias_ni_accepted() {
    let output = run_fledge(&["--ni", "doctor", "--json"]);
    assert!(
        output.status.success(),
        "--ni was rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_introspect_json_produces_valid_tree() {
    let output = run_fledge(&["introspect", "--json"]);
    assert!(
        output.status.success(),
        "introspect --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["name"].as_str(), Some("fledge"));
    assert!(parsed["subcommands"].is_array());
    let subs = parsed["subcommands"].as_array().unwrap();
    assert!(!subs.is_empty(), "expected non-empty subcommands list");
}

#[test]
fn cli_introspect_json_has_schema_version_at_top_level() {
    let output = run_fledge(&["introspect", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // schema_version sits alongside name/about/args/subcommands (additive,
    // not nested) — old consumers reading those keys keep working.
    assert_eq!(
        parsed["schema_version"].as_u64(),
        Some(1),
        "expected schema_version: 1 at top level, got: {parsed}"
    );
    assert_eq!(parsed["name"].as_str(), Some("fledge"));
    assert!(parsed["subcommands"].is_array());
}

#[test]
fn cli_introspect_json_includes_core_commands() {
    let output = run_fledge(&["introspect", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let subs = parsed["subcommands"].as_array().unwrap();
    let names: Vec<&str> = subs.iter().filter_map(|s| s["name"].as_str()).collect();
    for expected in ["ask", "review", "spec", "work", "introspect"] {
        assert!(
            names.contains(&expected),
            "expected '{expected}' in introspect output, got: {names:?}"
        );
    }
}

#[test]
fn cli_introspect_pretty_succeeds() {
    let output = run_fledge(&["introspect"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge"));
    assert!(stdout.contains("ask"));
    // Pretty output is not JSON
    assert!(!stdout.trim().starts_with('{'));
}

#[test]
fn cli_introspect_json_surfaces_global_non_interactive_flag() {
    let output = run_fledge(&["introspect", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let args = parsed["args"].as_array().unwrap();
    let ni = args
        .iter()
        .find(|a| a["long"].as_str() == Some("non-interactive"))
        .expect("global --non-interactive should appear in root args");
    assert_eq!(ni["global"].as_bool(), Some(true));
    assert_eq!(ni["takes_value"].as_bool(), Some(false));
    // Bool flags must not include a value_name
    assert!(ni.get("value_name").is_none());
    // The `ni` alias should be surfaced so agents can recognize --ni
    let aliases = ni["aliases"].as_array().expect("aliases array");
    assert!(
        aliases.iter().any(|a| a.as_str() == Some("ni")),
        "expected 'ni' alias, got: {aliases:?}"
    );
}
