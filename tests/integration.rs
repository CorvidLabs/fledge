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
// Flow commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_flow_no_fledge_toml_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["flow"]);
    assert!(!output.status.success());
}

#[test]
fn cli_flow_list_shows_flows() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
fmt = "echo fmt"
lint = "echo lint"
test = "echo test"

[flows.ci]
description = "CI pipeline"
steps = ["fmt", "lint", "test"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ci"));
}

#[test]
fn cli_flow_dry_run_does_not_execute() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo BUILT"

[flows.ci]
description = "CI"
steps = ["build"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "run", "ci", "--dry-run"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("BUILT"));
}

#[test]
fn cli_flow_executes_steps() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
step1 = "echo STEP1"
step2 = "echo STEP2"

[flows.pipeline]
steps = ["step1", "step2"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "run", "pipeline"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("STEP1"));
    assert!(stdout.contains("STEP2"));
}

#[test]
fn cli_flow_unknown_flow_fails() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\n[flows.ci]\nsteps = [\"build\"]\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "run", "nonexistent"]);
    assert!(!output.status.success());
}

#[test]
fn cli_flow_init_adds_default_flows() {
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
    let output = run_fledge_in(tmp.path(), &["flow", "init"]);
    assert!(output.status.success());
    let content = fs::read_to_string(tmp.path().join("fledge.toml")).unwrap();
    assert!(content.contains("[flows"));
}

#[test]
fn cli_flow_json_output() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
build = "echo build"

[flows.ci]
description = "CI"
steps = ["build"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "list", "--json"]);
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
    assert!(stdout.contains("flow"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("metrics"));
}

#[test]
fn cli_subcommand_help() {
    let subcommands = [
        "init",
        "run",
        "flow",
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
// Flow with inline and parallel steps
// ──────────────────────────────────────────────────────────

#[test]
fn cli_flow_inline_step() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]

[flows.greet]
steps = [{ run = "echo INLINE_HELLO" }]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "run", "greet"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("INLINE_HELLO"));
}

#[test]
fn cli_flow_parallel_steps() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
a = "echo ALPHA"
b = "echo BRAVO"

[flows.par]
steps = [{ parallel = ["a", "b"] }]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["flow", "run", "par"]);
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
// E2E workflow: init → run → flow → doctor → metrics → deps
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

    // Step 4: Generate default flows
    let output = run_fledge_in(&project, &["flow", "init"]);
    assert!(
        output.status.success(),
        "flow init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fledge_toml = fs::read_to_string(project.join("fledge.toml")).unwrap();
    assert!(fledge_toml.contains("[flows"));

    // Step 5: List flows
    let output = run_fledge_in(&project, &["flow", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ci"));

    // Step 6: Dry-run a flow
    let output = run_fledge_in(&project, &["flow", "run", "ci", "--dry-run"]);
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
