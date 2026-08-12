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
fn cli_run_init_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--init", "--json"]);
    assert!(
        output.status.success(),
        "run --init --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("run --init --json must emit JSON, got prose. error: {e}\nstdout:\n{stdout}")
    });
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("run_init"));
    assert_eq!(parsed["file"].as_str(), Some("fledge.toml"));
    assert!(parsed["files_created"].is_array());
}

#[test]
fn cli_run_list_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nbuild = \"echo build\"\ntest = \"echo test\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "--list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("run_list"));
    assert!(parsed["tasks"].is_array());
    assert_eq!(parsed["tasks"].as_array().unwrap().len(), 2);
}

#[test]
fn cli_run_task_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nhello = \"echo hi\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "hello", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("run_task"));
    assert_eq!(parsed["task"].as_str(), Some("hello"));
    assert_eq!(parsed["success"].as_bool(), Some(true));
    assert_eq!(parsed["exit_code"].as_i64(), Some(0));
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

// `--stream` (live child output)
// ──────────────────────────────────────────────────────────

/// Default `--json` stays buffered: nothing is mirrored, the envelope holds it
/// all. This is the backward-compatibility guard for the new flag.
#[test]
fn cli_run_json_buffers_by_default() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nnoisy = \"echo OUT-MARKER; echo ERR-MARKER 1>&2\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "noisy", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["stdout"].as_str().unwrap().contains("OUT-MARKER"));
    assert!(parsed["stderr"].as_str().unwrap().contains("ERR-MARKER"));
    assert!(
        !stderr.contains("OUT-MARKER") && !stderr.contains("ERR-MARKER"),
        "buffered mode must not mirror child output, got stderr: {stderr}"
    );
}

/// `--stream --json`: child bytes are mirrored to fledge's stderr AND still
/// captured in the envelope. Note the test harness pipes both streams, so this
/// also pins the non-TTY behaviour — the flag forwards regardless.
#[test]
fn cli_run_stream_json_mirrors_and_still_captures() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nnoisy = \"echo OUT-MARKER; echo ERR-MARKER 1>&2\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "noisy", "--json", "--stream"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["action"].as_str(), Some("run_task"));
    assert!(
        parsed["stdout"].as_str().unwrap().contains("OUT-MARKER"),
        "envelope must still carry stdout"
    );
    assert!(
        parsed["stderr"].as_str().unwrap().contains("ERR-MARKER"),
        "envelope must still carry stderr"
    );
    assert!(
        stderr.contains("OUT-MARKER"),
        "child stdout must be mirrored to fledge stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("ERR-MARKER"),
        "child stderr must be mirrored to fledge stderr, got: {stderr}"
    );
}

/// The whole reason mirroring targets stderr: stdout must remain one parseable
/// JSON document even when the child itself prints JSON-looking text.
#[test]
fn cli_run_stream_json_stdout_stays_pure_json() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nimposter = \"echo '{\\\"action\\\":\\\"not-the-envelope\\\"}'\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "imposter", "--json", "--stream"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must be a single JSON doc: {e}\n{stdout}"));
    assert_eq!(parsed["action"].as_str(), Some("run_task"));
    assert!(parsed["stdout"]
        .as_str()
        .unwrap()
        .contains("not-the-envelope"));
}

/// Exit code propagation is identical in both modes.
#[test]
fn cli_run_stream_json_failing_task_reports_exit_code() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nfail = \"echo BEFORE-FAIL; exit 3\"\n",
    )
    .unwrap();

    let streamed = run_fledge_in(tmp.path(), &["run", "fail", "--json", "--stream"]);
    assert!(!streamed.status.success());
    let stdout = String::from_utf8(streamed.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["exit_code"].as_i64(), Some(3));
    assert_eq!(parsed["success"].as_bool(), Some(false));
    assert!(parsed["stdout"].as_str().unwrap().contains("BEFORE-FAIL"));

    let buffered = run_fledge_in(tmp.path(), &["run", "fail", "--json"]);
    assert!(!buffered.status.success());
    let buffered_parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(buffered.stdout).unwrap()).unwrap();
    assert_eq!(
        buffered_parsed["exit_code"], parsed["exit_code"],
        "exit code must not depend on the output mode"
    );
    assert_eq!(buffered_parsed["stdout"], parsed["stdout"]);
}

/// `--stream` without `--json` is accepted and behaves exactly like the
/// existing human-readable run (which already inherits the terminal).
#[test]
fn cli_run_stream_without_json_is_accepted() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nhello = \"echo hello-streaming\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "hello", "--stream"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello-streaming"));
    assert!(
        stdout.contains("Running task: hello"),
        "fledge's own task summary must remain, got: {stdout}"
    );
}

/// A streamed failing task in human mode still exits non-zero.
#[test]
fn cli_run_stream_without_json_failing_task_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nfail = \"exit 4\"\n",
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "fail", "--stream"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("exit code 4"), "got: {stderr}");
}

/// Streaming is an output mode, so dependencies stream too.
#[test]
fn cli_run_stream_json_applies_to_dependencies() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        r#"[tasks]
prep = "echo DEP-MARKER"

[tasks.build]
cmd = "echo BUILD-MARKER"
deps = ["prep"]
"#,
    )
    .unwrap();
    let output = run_fledge_in(tmp.path(), &["run", "build", "--json", "--stream"]);
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("DEP-MARKER"),
        "dependency output must stream too, got: {stderr}"
    );
    assert!(stderr.contains("BUILD-MARKER"));
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
