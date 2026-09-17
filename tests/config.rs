mod common;
use common::*;

// Every test here runs under `TempEnv`: `config list` and `config get` read the
// config that `config path` resolves, so on a bare `run_fledge` they report the
// developer's real `~/.config/fledge/config.toml`.

// Config commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_config_path_shows_path() {
    // `FLEDGE_CONFIG_DIR` is removed on purpose: this test is about the
    // `dirs::config_dir()` derivation, which the override would short-circuit.
    // `TempEnv`'s temp HOME/XDG_CONFIG_HOME still keep it off the real one.
    let output = TempEnv::new()
        .command()
        .env_remove("FLEDGE_CONFIG_DIR")
        .args(["config", "path"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") && stdout.contains("config.toml"));
}

#[test]
fn cli_config_list_succeeds() {
    let output = TempEnv::new().run(&["config", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("defaults.author"));
    assert!(stdout.contains("defaults.license"));
    assert!(stdout.contains("templates.paths"));
}

#[test]
fn cli_config_get_unknown_key_fails() {
    let output = TempEnv::new().run(&["config", "get", "nonexistent.key"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unknown config key"));
}

#[test]
fn cli_config_get_valid_key_succeeds() {
    let output = TempEnv::new().run(&["config", "get", "defaults.license"]);
    assert!(output.status.success());
}

#[test]
fn cli_config_set_unknown_key_fails() {
    let output = TempEnv::new().run(&["config", "set", "bad.key", "value"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unknown config key"));
}

// ──────────────────────────────────────────────────────────
