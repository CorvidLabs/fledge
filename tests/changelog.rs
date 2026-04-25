mod common;
use common::*;

use tempfile::TempDir;

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
        "templates",
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
            "templates",
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
    let output = run_fledge(&["templates", "list"]);
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
