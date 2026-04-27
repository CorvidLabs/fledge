mod common;
use common::*;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// Release command (requires git repo)
// ──────────────────────────────────────────────────────────

fn init_git_repo(dir: &std::path::Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn cli_release_dry_run_json_emits_envelope() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = run_fledge_in(tmp.path(), &["release", "0.2.0", "--dry-run", "--json"]);
    assert!(
        output.status.success(),
        "release --dry-run --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("release --dry-run --json must emit JSON. error: {e}\nstdout:\n{stdout}")
    });
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("release"));
    assert_eq!(parsed["dry_run"].as_bool(), Some(true));
    assert_eq!(parsed["version"].as_str(), Some("0.2.0"));
    assert_eq!(parsed["tag"].as_str(), Some("v0.2.0"));
    assert_eq!(parsed["will_push"].as_bool(), Some(false));
    assert!(parsed["files_to_bump"].is_array());
    let files = parsed["files_to_bump"].as_array().unwrap();
    assert!(
        files.iter().any(|f| f.as_str() == Some("Cargo.toml")),
        "expected Cargo.toml in files_to_bump, got: {files:?}"
    );
}

#[test]
fn cli_release_dry_run_json_no_bump_flag() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = run_fledge_in(
        tmp.path(),
        &["release", "0.2.0", "--dry-run", "--json", "--no-bump"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["no_bump"].as_bool(), Some(true));
    assert_eq!(
        parsed["files_to_bump"].as_array().unwrap().len(),
        0,
        "no_bump should result in empty files_to_bump"
    );
}

#[test]
fn cli_release_dry_run_json_with_pre_lane_emits_single_envelope() {
    // Regression: --pre-lane combined with --json must not emit a second
    // (lane) JSON envelope on stdout. The release envelope is the only thing
    // a consumer should have to parse.
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\necho = \"echo hi\"\n\n[lanes.smoke]\ndescription = \"smoke\"\nsteps = [\"echo\"]\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = run_fledge_in(
        tmp.path(),
        &[
            "release",
            "0.2.0",
            "--dry-run",
            "--json",
            "--pre-lane",
            "smoke",
        ],
    );
    assert!(
        output.status.success(),
        "release --dry-run --json --pre-lane failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("expected exactly one JSON envelope on stdout. error: {e}\nstdout:\n{stdout}")
    });
    assert_eq!(parsed["action"].as_str(), Some("release"));
    assert_eq!(parsed["dry_run"].as_bool(), Some(true));
}

#[test]
fn cli_release_json_with_failing_pre_lane_bails_with_plain_stderr() {
    // Failure path: lane fails -> release bails. Stdout stays empty (no
    // partial envelope), stderr carries a plain-text error, exit is non-zero.
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("fledge.toml"),
        "[tasks]\nfail = \"exit 1\"\n\n[lanes.busted]\ndescription = \"busted\"\nsteps = [\"fail\"]\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = run_fledge_in(
        tmp.path(),
        &[
            "release",
            "0.2.0",
            "--json",
            "--pre-lane",
            "busted",
            "--no-bump",
            "--no-tag",
            "--no-changelog",
        ],
    );
    assert!(
        !output.status.success(),
        "expected non-zero exit when pre-lane fails"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.trim().is_empty(),
        "expected empty stdout on lane failure, got: {stdout}"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("busted"),
        "expected lane name in stderr, got: {stderr}"
    );
}

// ──────────────────────────────────────────────────────────
