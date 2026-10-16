//! `fledge config set` must not print values for keys `Config::is_secret_key`
//! already treats as secret. Each case spawns this checkout's binary in its own
//! disposable home, XDG directories, `FLEDGE_CONFIG_DIR`, and working directory,
//! with inherited GitHub and provider credentials removed.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const SECRET_KEYS: &[(&str, &str, bool)] = &[
    ("github.token", "F1_SYNTHETIC_GITHUB_TOKEN_MARKER", true),
    (
        "ai.anthropic.api_key",
        "F1_SYNTHETIC_ANTHROPIC_API_KEY_MARKER",
        true,
    ),
    (
        "ai.openai.api_key",
        "F1_SYNTHETIC_OPENAI_API_KEY_MARKER",
        true,
    ),
    (
        "ai.claude.api_key",
        "F1_SYNTHETIC_CLAUDE_API_KEY_MARKER",
        false,
    ),
    (
        "ai.ollama.api_key",
        "F1_SYNTHETIC_OLLAMA_API_KEY_MARKER",
        true,
    ),
];

const CLEARED_ENV: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "OLLAMA_API_KEY",
    "OPENROUTER_API_KEY",
    "GEMINI_API_KEY",
    "DEEPSEEK_API_KEY",
    "GROQ_API_KEY",
    "MISTRAL_API_KEY",
    "XAI_API_KEY",
    "TOGETHER_API_KEY",
    "FLEDGE_AI_PROVIDER",
    "FLEDGE_AI_MODEL",
    "FLEDGE_AI_HOST",
    "FLEDGE_GITHUB_TOKEN",
    "GITHUB_TOKEN",
    "GH_TOKEN",
];

struct Isolated {
    home: TempDir,
    xdg_config: TempDir,
    xdg_cache: TempDir,
    xdg_data: TempDir,
    config: TempDir,
    cwd: TempDir,
}

impl Isolated {
    fn new() -> Self {
        Self {
            home: TempDir::new().expect("disposable HOME"),
            xdg_config: TempDir::new().expect("disposable XDG_CONFIG_HOME"),
            xdg_cache: TempDir::new().expect("disposable XDG_CACHE_HOME"),
            xdg_data: TempDir::new().expect("disposable XDG_DATA_HOME"),
            config: TempDir::new().expect("disposable FLEDGE_CONFIG_DIR"),
            cwd: TempDir::new().expect("disposable current directory"),
        }
    }

    fn config_file(&self) -> std::path::PathBuf {
        self.config.path().join("config.toml")
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fledge"));
        cmd.current_dir(self.cwd.path())
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.xdg_config.path())
            .env("XDG_CACHE_HOME", self.xdg_cache.path())
            .env("XDG_DATA_HOME", self.xdg_data.path())
            .env("FLEDGE_CONFIG_DIR", self.config.path())
            .env("FLEDGE_NON_INTERACTIVE", "1")
            .env("NO_COLOR", "1")
            .env_remove("CLICOLOR_FORCE")
            .env_remove("FORCE_COLOR");
        for key in CLEARED_ENV {
            cmd.env_remove(*key);
        }
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(args)
            .output()
            .expect("spawn checkout fledge")
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_marker_absent(step: &str, key: &str, marker: &str, stdout: &str, stderr: &str) {
    assert!(
        !stdout.contains(marker) && !stderr.contains(marker),
        "{step} echoed the synthetic marker for {key}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Read a dotted string from the pretty TOML `config set` writes.
/// Handles `[a.b]` headers and `key = "value"` / `a.b = "value"` lines.
fn persisted_string(document: &str, dotted: &str) -> Option<String> {
    let mut table: Vec<String> = Vec::new();
    for raw in document.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            table = line[1..line.len() - 1]
                .split('.')
                .map(|part| part.trim().to_string())
                .collect();
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let mut path = table.clone();
        path.extend(
            raw_key
                .trim()
                .split('.')
                .map(|part| part.trim().to_string()),
        );
        if path.join(".") == dotted {
            return basic_or_literal_string(raw_value.trim());
        }
    }
    None
}

fn basic_or_literal_string(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 {
        let quote = bytes[0];
        if (quote == b'"' || quote == b'\'') && *bytes.last().expect("len checked") == quote {
            return Some(raw[1..raw.len() - 1].to_string());
        }
    }
    None
}

fn assert_persisted(config_file: &Path, key: &str, marker: &str) {
    let document = std::fs::read_to_string(config_file).unwrap_or_else(|error| {
        panic!("owned config.toml missing after set {key}: {error}");
    });
    let stored = persisted_string(&document, key);
    assert_eq!(
        stored.as_deref(),
        Some(marker),
        "owned config.toml did not persist the exact value for {key}\n{document}"
    );
}

fn assert_secret_round_trip(key: &str, marker: &str, listed: bool) {
    let isolated = Isolated::new();
    let config_file = isolated.config_file();
    assert!(
        !config_file.exists(),
        "disposable config dir was not empty before set"
    );

    let set = isolated.run(&["config", "set", key, marker]);
    let set_out = text(&set.stdout);
    let set_err = text(&set.stderr);
    assert!(
        set.status.success(),
        "config set {key} failed\nstdout:\n{set_out}\nstderr:\n{set_err}"
    );
    assert_marker_absent("config set", key, marker, &set_out, &set_err);
    assert!(
        set_out.contains(key) && set_out.contains("***") && set_out.contains("Set"),
        "config set did not confirm {key} with a mask\nstdout:\n{set_out}"
    );
    assert_persisted(&config_file, key, marker);

    let get = isolated.run(&["config", "get", key]);
    let get_out = text(&get.stdout);
    let get_err = text(&get.stderr);
    assert!(
        get.status.success(),
        "config get {key} failed\nstdout:\n{get_out}\nstderr:\n{get_err}"
    );
    assert_marker_absent("config get", key, marker, &get_out, &get_err);
    assert_eq!(
        get_out.trim(),
        "***",
        "config get did not mask {key}\nstdout:\n{get_out}"
    );

    let list = isolated.run(&["config", "list"]);
    let list_out = text(&list.stdout);
    let list_err = text(&list.stderr);
    assert!(
        list.status.success(),
        "config list failed after set {key}\nstdout:\n{list_out}\nstderr:\n{list_err}"
    );
    assert_marker_absent("config list", key, marker, &list_out, &list_err);
    let listed_row = list_out.lines().any(|line| line.contains(key));
    if listed {
        assert!(
            listed_row,
            "config list omitted the stored secret key {key}\nstdout:\n{list_out}"
        );
    }
    for line in list_out.lines().filter(|line| line.contains(key)) {
        assert!(
            line.contains("***"),
            "config list row for {key} was not masked\n{line}"
        );
    }
}

#[test]
fn config_set_masks_github_token() {
    let (key, marker, listed) = SECRET_KEYS[0];
    assert_secret_round_trip(key, marker, listed);
}

#[test]
fn config_set_masks_anthropic_api_key() {
    let (key, marker, listed) = SECRET_KEYS[1];
    assert_secret_round_trip(key, marker, listed);
}

#[test]
fn config_set_masks_openai_api_key() {
    let (key, marker, listed) = SECRET_KEYS[2];
    assert_secret_round_trip(key, marker, listed);
}

#[test]
fn config_set_masks_claude_api_key() {
    let (key, marker, listed) = SECRET_KEYS[3];
    assert_secret_round_trip(key, marker, listed);
}

#[test]
fn config_set_masks_ollama_api_key() {
    let (key, marker, listed) = SECRET_KEYS[4];
    assert_secret_round_trip(key, marker, listed);
}

#[test]
fn config_set_prints_nonsecret_defaults_author() {
    let key = "defaults.author";
    let marker = "F1_SYNTHETIC_AUTHOR_VISIBLE";
    let isolated = Isolated::new();
    let set = isolated.run(&["config", "set", key, marker]);
    let set_out = text(&set.stdout);
    let set_err = text(&set.stderr);
    assert!(
        set.status.success(),
        "config set {key} failed\nstdout:\n{set_out}\nstderr:\n{set_err}"
    );
    assert!(
        set_out.contains(key) && set_out.contains(marker) && set_out.contains("Set"),
        "non-secret confirmation hid {key}\nstdout:\n{set_out}"
    );
    assert_persisted(&isolated.config_file(), key, marker);
}

#[test]
fn config_set_unknown_key_does_not_echo_or_write() {
    let key = "f1.not.a.config.key";
    let marker = "F1_SYNTHETIC_UNKNOWN_VALUE_MARKER";
    let isolated = Isolated::new();
    let config_file = isolated.config_file();
    let before = std::fs::read(&config_file).ok();
    assert!(
        before.is_none(),
        "disposable config existed before the call"
    );

    let set = isolated.run(&["config", "set", key, marker]);
    let set_out = text(&set.stdout);
    let set_err = text(&set.stderr);
    assert!(
        !set.status.success(),
        "unknown key was accepted\nstdout:\n{set_out}\nstderr:\n{set_err}"
    );
    assert_marker_absent("config set", key, marker, &set_out, &set_err);
    assert!(
        set_err.contains("Unknown config key"),
        "unknown-key failure did not name the key\nstderr:\n{set_err}"
    );
    let after = std::fs::read(&config_file).ok();
    assert_eq!(
        before, after,
        "unknown key changed owned config.toml\nbefore: {before:?}\nafter: {after:?}"
    );
}
