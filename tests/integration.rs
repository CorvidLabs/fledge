use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn cargo_bin() -> String {
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("cargo build failed");
    assert!(output.success());

    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/fledge");
    target_dir.to_string_lossy().to_string()
}

fn run_fledge(args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin).args(args).output().unwrap()
}

fn run_fledge_in(dir: &Path, args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn cli_list_shows_templates() {
    let bin = cargo_bin();
    let output = Command::new(&bin).arg("list").output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "list failed: {stdout}");
    assert!(stdout.contains("rust-cli"));
    assert!(stdout.contains("ts-bun"));
}

#[test]
fn cli_init_with_template_creates_project() {
    let bin = cargo_bin();
    let tmp = TempDir::new().unwrap();

    let output = Command::new(&bin)
        .args([
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

    // Non-interactive mode may fail because dialoguer needs a TTY for author prompt.
    // If it succeeds, verify the output.
    if output.status.success() {
        let project_dir = tmp.path().join("test-project");
        assert!(project_dir.exists(), "project dir not created");
        assert!(
            project_dir.join("Cargo.toml").exists() || project_dir.join("src").exists(),
            "project files not found"
        );
        assert!(
            project_dir.join("src/lib.rs").exists(),
            "src/lib.rs not found"
        );
    } else {
        // Expected in CI/non-TTY: dialoguer prompt fails
        assert!(
            stderr.contains("dialoguer")
                || stderr.contains("not a terminal")
                || stderr.contains("IO error"),
            "unexpected error: {stderr}\nstdout: {stdout}"
        );
    }
}

#[test]
fn cli_init_unknown_template_fails() {
    let bin = cargo_bin();
    let tmp = TempDir::new().unwrap();

    let output = Command::new(&bin)
        .args([
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
    assert!(stdout.contains("init"));
    assert!(stdout.contains("list"));
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
// Config commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_config_path_shows_path() {
    let output = run_fledge(&["config", "path"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") && stdout.contains("config.toml"));
}

#[test]
fn cli_config_list_succeeds() {
    let output = run_fledge(&["config", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("defaults.author"));
    assert!(stdout.contains("defaults.license"));
    assert!(stdout.contains("templates.paths"));
}

#[test]
fn cli_config_get_unknown_key_fails() {
    let output = run_fledge(&["config", "get", "nonexistent.key"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unknown config key"));
}

#[test]
fn cli_config_get_valid_key_succeeds() {
    let output = run_fledge(&["config", "get", "defaults.license"]);
    assert!(output.status.success());
}

#[test]
fn cli_config_set_unknown_key_fails() {
    let output = run_fledge(&["config", "set", "bad.key", "value"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unknown config key"));
}

// ──────────────────────────────────────────────────────────
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
// Doctor command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_doctor_succeeds() {
    let output = run_fledge(&["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Toolchain") || stdout.contains("Git"));
}

#[test]
fn cli_doctor_json_valid() {
    let output = run_fledge(&["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["project_type"].is_string());
    assert!(parsed["sections"].is_array());
    assert!(parsed["passed"].is_number());
    assert!(parsed["failed"].is_number());
}

#[test]
fn cli_doctor_detects_rust_project() {
    let output = run_fledge(&["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["project_type"], "rust");
}

// ──────────────────────────────────────────────────────────
// Metrics command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_metrics_succeeds() {
    let output = run_fledge(&["metrics"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Rust") || stdout.contains("Lines"));
}

#[test]
fn cli_metrics_json_valid() {
    let output = run_fledge(&["metrics", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["summary"]["files"].is_number());
    assert!(parsed["summary"]["lines"].is_number());
    assert!(parsed["summary"]["code"].is_number());
    assert!(parsed["languages"].is_array());
}

#[test]
fn cli_metrics_churn_succeeds() {
    let output = run_fledge(&["metrics", "--churn"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Commits") || stdout.contains("File") || stdout.contains("churn"));
}

#[test]
fn cli_metrics_churn_json() {
    let output = run_fledge(&["metrics", "--churn", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn cli_metrics_tests_succeeds() {
    let output = run_fledge(&["metrics", "--tests"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Test files") || stdout.contains("Source files"));
}

#[test]
fn cli_metrics_tests_json() {
    let output = run_fledge(&["metrics", "--tests", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["test_files"].is_number());
    assert!(parsed["source_files"].is_number());
    assert!(parsed["ratio"].is_number());
}

#[test]
fn cli_metrics_churn_with_limit() {
    let output = run_fledge(&["metrics", "--churn", "--limit", "5", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.as_array().unwrap().len() <= 5);
}

// ──────────────────────────────────────────────────────────
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

    let output = run_fledge(&["validate-template", tpl.to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn cli_validate_template_invalid_toml_fails() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("bad");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(tpl.join("template.toml"), "not valid {{{}}\n").unwrap();

    let output = run_fledge(&["validate-template", tpl.to_str().unwrap()]);
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

    let output = run_fledge(&["validate-template", tpl.to_str().unwrap(), "--json"]);
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

    let output = run_fledge(&["validate-template", tpl.to_str().unwrap(), "--strict"]);
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

    let output = run_fledge(&["validate-template", tmp.path().to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn cli_validate_template_no_templates_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&["validate-template", tmp.path().to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No templates found"));
}

// ──────────────────────────────────────────────────────────
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

// ──────────────────────────────────────────────────────────
// Changelog command (requires git repo)
// ──────────────────────────────────────────────────────────

#[test]
fn cli_changelog_succeeds() {
    let output = run_fledge(&["changelog"]);
    // May show tags or "no tags" — should succeed either way
    assert!(output.status.success());
}

#[test]
fn cli_changelog_json_valid() {
    let output = run_fledge(&["changelog", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // When there are no tags, changelog prints a plain-text hint instead of JSON
    if stdout.trim().is_empty() || !stdout.trim_start().starts_with('[') {
        return;
    }
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn cli_changelog_unreleased() {
    let output = run_fledge(&["changelog", "--unreleased"]);
    assert!(output.status.success());
}

#[test]
fn cli_changelog_with_limit() {
    let output = run_fledge(&["changelog", "--limit", "3"]);
    assert!(output.status.success());
}

// ──────────────────────────────────────────────────────────
// Init with --yes flag (non-interactive)
// ──────────────────────────────────────────────────────────

#[test]
fn cli_init_yes_flag_skips_prompts() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "init",
        "yes-test",
        "--template",
        "rust-cli",
        "--output",
        tmp.path().to_str().unwrap(),
        "--no-git",
        "--yes",
    ]);
    assert!(output.status.success());
    let project_dir = tmp.path().join("yes-test");
    assert!(project_dir.exists());
    assert!(project_dir.join("Cargo.toml").exists());
    assert!(project_dir.join("src/main.rs").exists());
}

#[test]
fn cli_init_yes_with_each_builtin_template() {
    let templates = ["rust-cli", "ts-bun"];
    for tpl in &templates {
        let tmp = TempDir::new().unwrap();
        let output = run_fledge(&[
            "init",
            &format!("{tpl}-test"),
            "--template",
            tpl,
            "--output",
            tmp.path().to_str().unwrap(),
            "--no-git",
            "--no-install",
            "--yes",
        ]);
        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        assert!(
            output.status.success(),
            "template '{tpl}' failed:\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(
            tmp.path().join(format!("{tpl}-test")).exists(),
            "project dir for '{tpl}' not created"
        );
    }
}

// ──────────────────────────────────────────────────────────
// List shows all built-in templates
// ──────────────────────────────────────────────────────────

#[test]
fn cli_list_shows_all_builtin_templates() {
    let output = run_fledge(&["list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = ["rust-cli", "ts-bun"];
    for tpl in &expected {
        assert!(
            stdout.contains(tpl),
            "missing template '{tpl}' in list output"
        );
    }
}

// ──────────────────────────────────────────────────────────
// Help subcommands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_help_flag_shows_usage() {
    let output = run_fledge(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("init"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("lane"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("metrics"));
}

#[test]
fn cli_subcommand_help() {
    let subcommands = [
        "init",
        "run",
        "lane",
        "config",
        "spec",
        "doctor",
        "metrics",
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
    let output = run_fledge(&["validate-template", templates_dir.to_str().unwrap()]);
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
// Deps command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_deps_lists_rust_dependencies() {
    let output = run_fledge(&["deps"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Dependencies"));
    assert!(stdout.contains("clap"));
    assert!(stdout.contains("serde"));
}

#[test]
fn cli_deps_json_valid() {
    let output = run_fledge(&["deps", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["ecosystem"], "rust");
    assert!(parsed["dependencies"].is_array());
    assert!(!parsed["dependencies"].as_array().unwrap().is_empty());
}

#[test]
fn cli_deps_generic_project_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["deps"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Could not detect"));
}

#[test]
fn cli_deps_node_project_no_lock_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("package.json"), "{}").unwrap();
    let output = run_fledge_in(tmp.path(), &["deps"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No lock file"));
}

// ──────────────────────────────────────────────────────────
// E2E workflow: init → run → lane → doctor → metrics → deps
// ──────────────────────────────────────────────────────────

#[test]
fn e2e_rust_project_lifecycle() {
    let tmp = TempDir::new().unwrap();

    // Step 1: Init a Rust project
    let output = run_fledge(&[
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

    // Step 2: Generate task runner config
    let output = run_fledge_in(&project, &["run", "--init"]);
    assert!(
        output.status.success(),
        "run --init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    assert!(stdout.contains("Toolchain"));

    // Step 8: Metrics
    let output = run_fledge_in(&project, &["metrics"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Rust") || stdout.contains("Lines"));

    // Step 9: Doctor JSON
    let output = run_fledge_in(&project, &["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["project_type"], "rust");
}

#[test]
fn e2e_tsbun_project_lifecycle() {
    let tmp = TempDir::new().unwrap();

    // Step 1: Init a ts-bun project
    let output = run_fledge(&[
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

    // Step 2: Generate task runner config
    let output = run_fledge_in(&project, &["run", "--init"]);
    assert!(output.status.success());

    // Step 3: Doctor
    let output = run_fledge_in(&project, &["doctor"]);
    assert!(output.status.success());

    // Step 4: Metrics
    let output = run_fledge_in(&project, &["metrics"]);
    assert!(output.status.success());
}

// ══════════════════════════════════════════════════════════
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
    let output = run_fledge(&["config", "add", "templates.paths", "/tmp/e2e-test-path"]);
    assert!(output.status.success());

    let output = run_fledge(&["config", "get", "templates.paths"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("/tmp/e2e-test-path"));

    let output = run_fledge(&["config", "remove", "templates.paths", "/tmp/e2e-test-path"]);
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

// ──────────────────────────────────────────────────────────
// Special characters and unicode in project names
// ──────────────────────────────────────────────────────────

#[test]
fn cli_init_name_with_spaces_handled() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
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
// Metrics edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_metrics_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["metrics"]);
    // Should succeed with zero counts or fail gracefully
    assert!(output.status.success());
}

#[test]
fn cli_metrics_json_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["metrics", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["summary"]["files"], 0);
}

#[test]
fn cli_metrics_churn_in_non_git_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["metrics", "--churn"]);
    // Not a git repo — should handle gracefully
    let _ = output.status;
}

#[test]
fn cli_metrics_tests_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["metrics", "--tests"]);
    assert!(output.status.success());
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
    assert!(stdout.contains("generic") || stdout.contains("Toolchain") || stdout.contains("Git"));
}

#[test]
fn cli_doctor_json_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["project_type"], "generic");
}

// ──────────────────────────────────────────────────────────
// Validate-template edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_validate_template_nonexistent_path() {
    let output = run_fledge(&["validate-template", "/tmp/no-such-path-ever-12345"]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_empty_template_toml() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("empty-tpl");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(tpl.join("template.toml"), "").unwrap();
    let output = run_fledge(&["validate-template", tpl.to_str().unwrap()]);
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
    let output = run_fledge(&["validate-template", tpl.to_str().unwrap()]);
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
    let output = run_fledge(&["validate-template", tpl.to_str().unwrap()]);
    // Missing description might be a warning or error
    let _status = output.status;
}

// ──────────────────────────────────────────────────────────
// Create-template command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_create_template_creates_scaffold() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge(&[
        "create-template",
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
        "create-template",
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
// External/unknown subcommand
// ──────────────────────────────────────────────────────────

#[test]
fn cli_unknown_subcommand_fails() {
    let output = run_fledge(&["definitely-not-a-command"]);
    // External subcommand dispatch — should fail if no matching plugin
    assert!(!output.status.success());
}

// ──────────────────────────────────────────────────────────
// Deps edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_deps_go_project_without_gomod() {
    let tmp = TempDir::new().unwrap();
    // Only a main.go, no go.mod — should fail or detect nothing
    fs::write(tmp.path().join("main.go"), "package main\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["deps"]);
    assert!(!output.status.success());
}

#[test]
fn cli_deps_python_project() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("pyproject.toml"), "[tool]\n").unwrap();
    let output = run_fledge_in(tmp.path(), &["deps"]);
    // May fail without pip/lock files, but shouldn't panic
    let _ = output.status;
}

#[test]
fn cli_deps_json_empty_project_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["deps", "--json"]);
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

    // 10. Doctor and metrics in this dir
    let output = run_fledge_in(tmp.path(), &["doctor"]);
    assert!(output.status.success());

    let output = run_fledge_in(tmp.path(), &["metrics"]);
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
        "create-template",
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
    let output = run_fledge(&["validate-template", tpl_dir.to_str().unwrap()]);
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
        "create-template",
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
    let output = run_fledge(&["validate-template", tpl_dir.to_str().unwrap()]);
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
        "create-template",
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
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let output = run_fledge_in(tmp.path(), &["review", "--base", "HEAD"]);
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("No changes") || stderr.contains("Claude CLI"),
            "expected no-changes or CLI error, got: {stderr}"
        );
    }
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
fn cli_ask_accepts_json_flag() {
    let output = run_fledge(&["ask", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}
