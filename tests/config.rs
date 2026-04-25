mod common;
use common::*;

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
