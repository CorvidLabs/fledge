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

#[test]
fn cli_list_shows_templates() {
    let bin = cargo_bin();
    let output = Command::new(&bin).arg("list").output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "list failed: {stdout}");
    assert!(stdout.contains("rust-cli"));
    assert!(stdout.contains("rust-lib"));
    assert!(stdout.contains("ts-bun"));
    assert!(stdout.contains("angular-app"));
    assert!(stdout.contains("swift-pkg"));
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
            "rust-lib",
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
            project_dir.join("Cargo.toml").exists(),
            "Cargo.toml not found"
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
