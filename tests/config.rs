mod common;
use common::*;

// Every test here runs under `TempEnv`: `config list` and `config get` read the
// config that `config path` resolves, so on a bare `run_fledge` they report the
// developer's real `~/.config/fledge/config.toml`.

#[cfg(not(windows))]
#[test]
fn cli_config_home_path_guard_rejects_parent_traversal_and_outside_paths() {
    let env = TempEnv::new();
    let inside = env.home().join(".config/fledge/config.toml");
    assert_eq!(
        checked_home_config_path(inside.to_str().unwrap(), env.home()),
        inside
    );
    let traversal = env.home().join("../outside/config.toml");
    assert!(
        traversal.starts_with(env.home()),
        "control must pass lexical prefix"
    );
    for rejected in [
        traversal,
        env.home().with_extension("sibling").join("config.toml"),
        std::path::PathBuf::from("relative/config.toml"),
    ] {
        assert!(
            std::panic::catch_unwind(|| checked_home_config_path(
                rejected.to_str().unwrap(),
                env.home()
            ))
            .is_err(),
            "unsafe printed config path was accepted"
        );
    }
}

// Config commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_config_path_shows_path() {
    // `FLEDGE_CONFIG_DIR` is removed on purpose: this test is about the
    // `dirs::config_dir()` derivation, which the override would short-circuit.
    // `TempEnv`'s temp HOME/XDG_CONFIG_HOME still keep it off the real one.
    let env = TempEnv::new();
    let output = env
        .command()
        .env_remove("FLEDGE_CONFIG_DIR")
        .args(["config", "path"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let path = std::path::PathBuf::from(stdout.trim());
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("config.toml")
    );
    assert!(
        path.components()
            .any(|component| component.as_os_str() == "fledge"),
        "config path should include a fledge directory"
    );
    // Windows resolves this through a known-folder API that ignores HOME.
    // Asserting the prefix there would compare against the operator profile.
    // Do not print or read that path. Unix must stay inside the disposable HOME.
    #[cfg(not(windows))]
    assert!(
        path.starts_with(env.home()),
        "default config path escaped the disposable HOME"
    );
    #[cfg(windows)]
    drop(env);
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
// config init: child HOME + FLEDGE_CONFIG_DIR, never the operator file
// ──────────────────────────────────────────────────────────

/// Exact bytes `config init` wrote on a disposable override during the
/// 2026-10-12 reproduction. A drift here is a change in the file users get.
const DEFAULT_CONFIG_TOML: &str = "\
[defaults]
license = \"MIT\"

[templates]
paths = []
repos = []

[github]

[ai.ollama]

[trust]
orgs = []
users = []
";

const CORVIDLABS_CONFIG_TOML: &str = "\
[defaults]
author = \"CorvidLabs\"
github_org = \"CorvidLabs\"
license = \"MIT\"

[templates]
paths = []
repos = [\"CorvidLabs/fledge-templates\"]

[github]

[ai.ollama]

[trust]
orgs = []
users = []
";

#[cfg(not(windows))]
const HOME_SENTINEL: &str = "sentinel = \"G4-DO-NOT-TOUCH\"\n";

fn assert_success(output: &std::process::Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(not(windows))]
fn sentinel_bytes(paths: &[std::path::PathBuf]) -> Vec<Vec<u8>> {
    paths
        .iter()
        .map(|path| std::fs::read(path).expect("read sentinel"))
        .collect()
}

#[cfg(not(windows))]
fn assert_sentinels_unchanged(paths: &[std::path::PathBuf], before: &[Vec<u8>]) {
    for (path, expected) in paths.iter().zip(before) {
        let now = std::fs::read(path).expect("reread sentinel");
        assert_eq!(
            now,
            *expected,
            "home config sentinel changed at {}",
            path.display()
        );
    }
}

#[test]
fn cli_config_init_writes_override_preserves_sentinel_and_stays_offline() {
    // macOS CI and this checkout have sandbox-exec. A pass that skipped the
    // denial would not be the network assertion this test exists to make.
    if cfg!(target_os = "macos") {
        assert!(
            network_confinement_available(),
            "macOS config init coverage must run under sandbox-exec"
        );
    }

    let server = MockHttp::start(r#"{"ok":false}"#);
    let env = TempEnv::new().with_github_api_base(server.url());
    #[cfg(not(windows))]
    let sentinels = env.plant_home_sentinels(HOME_SENTINEL);
    #[cfg(not(windows))]
    let before = sentinel_bytes(&sentinels);

    let confined = env
        .command_network_confined()
        .env("OLLAMA_HOST", server.url())
        .args(["config", "init"])
        .output()
        .unwrap();
    assert_success(&confined, "network-confined config init");
    assert!(
        server.requests().is_empty(),
        "config init contacted the loopback stand-in: {:?}",
        server.requests()
    );

    let created = env.config_dir().join("config.toml");
    let body = std::fs::read_to_string(&created).expect("read created config");
    assert_eq!(body, DEFAULT_CONFIG_TOML);

    let path_out = env.run(&["config", "path"]);
    assert_success(&path_out, "config path");
    assert_eq!(
        String::from_utf8(path_out.stdout).unwrap().trim(),
        created.to_str().unwrap()
    );

    #[cfg(not(windows))]
    assert_sentinels_unchanged(&sentinels, &before);

    // A second child is not sandboxed, so a swallowed connection to the
    // GitHub or Ollama base would be recorded. The confined run above can
    // hide that, because the sandbox drops the packet before the stand-in
    // accepts it. Init refuses to overwrite, so this child gets its own dir.
    let open = TempEnv::new().with_github_api_base(server.url());
    let unconfined = open
        .command()
        .env("OLLAMA_HOST", server.url())
        .args(["config", "init"])
        .output()
        .unwrap();
    assert_success(&unconfined, "unconfined config init");
    assert_eq!(
        std::fs::read_to_string(open.config_dir().join("config.toml")).unwrap(),
        DEFAULT_CONFIG_TOML
    );
    assert!(
        server.requests().is_empty(),
        "config init contacted the loopback stand-in: {:?}",
        server.requests()
    );
}

/// Negative control for the sentinel test. With `FLEDGE_CONFIG_DIR` removed,
/// `config init` writes the home-derived path the sentinel occupies. If that
/// stopped being the write target, the "unchanged" assertion would be watching
/// a file init never touches.
#[cfg(not(windows))]
#[test]
fn cli_config_init_without_override_writes_the_disposable_home_config() {
    let env = TempEnv::new();
    let home_config = env.home_config_path();
    assert!(
        !home_config.exists(),
        "disposable home config should start absent"
    );

    let output = env
        .command()
        .env_remove("FLEDGE_CONFIG_DIR")
        .args(["config", "init"])
        .output()
        .unwrap();
    assert_success(&output, "config init without override");

    let body = std::fs::read_to_string(&home_config).expect("read home config");
    assert_eq!(body, DEFAULT_CONFIG_TOML);
    assert!(
        !env.config_dir().join("config.toml").exists(),
        "override dir must stay empty when FLEDGE_CONFIG_DIR is unset"
    );
}

#[test]
fn cli_config_init_rejects_a_non_directory_destination() {
    let env = TempEnv::new();
    #[cfg(not(windows))]
    let sentinels = env.plant_home_sentinels(HOME_SENTINEL);
    #[cfg(not(windows))]
    let before = sentinel_bytes(&sentinels);

    let bad = env.home().join("not-a-directory");
    std::fs::write(&bad, b"not a directory\n").unwrap();

    let output = env
        .command()
        .env("FLEDGE_CONFIG_DIR", &bad)
        .args(["config", "init"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "config init should fail when FLEDGE_CONFIG_DIR is a file, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!output.stderr.is_empty(), "expected an error on stderr");
    assert_eq!(std::fs::read(&bad).unwrap(), b"not a directory\n");
    #[cfg(not(windows))]
    assert_sentinels_unchanged(&sentinels, &before);
}

#[test]
fn cli_config_init_leaves_an_existing_config_unchanged() {
    let env = TempEnv::new();
    #[cfg(not(windows))]
    let sentinels = env.plant_home_sentinels(HOME_SENTINEL);
    #[cfg(not(windows))]
    let before = sentinel_bytes(&sentinels);

    let existing = env.config_dir().join("config.toml");
    let sentinel = b"license = \"SENTINEL-EXISTING\"\n";
    std::fs::write(&existing, sentinel).unwrap();

    let output = env.run(&["config", "init"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("already exists"),
        "expected an already-exists error, got: {stderr}"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), sentinel);
    #[cfg(not(windows))]
    assert_sentinels_unchanged(&sentinels, &before);
}

#[test]
fn cli_config_init_preset_writes_expected_file_and_preserves_sentinel() {
    let env = TempEnv::new();
    #[cfg(not(windows))]
    let sentinels = env.plant_home_sentinels(HOME_SENTINEL);
    #[cfg(not(windows))]
    let before = sentinel_bytes(&sentinels);

    let output = env
        .command_network_confined()
        .args(["config", "init", "--preset", "corvidlabs"])
        .output()
        .unwrap();
    assert_success(&output, "config init --preset corvidlabs");

    let created = env.config_dir().join("config.toml");
    let body = std::fs::read_to_string(&created).unwrap();
    assert_eq!(body, CORVIDLABS_CONFIG_TOML);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(created.to_str().unwrap()),
        "creation message should name the override file, got: {stdout}"
    );
    #[cfg(not(windows))]
    assert_sentinels_unchanged(&sentinels, &before);
}

// ──────────────────────────────────────────────────────────
