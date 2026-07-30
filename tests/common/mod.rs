//! Shared helpers for integration tests. Per-module test files at
//! `tests/<module>.rs` pull these in via `mod common; use common::*;`.

#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub fn cargo_bin() -> String {
    env!("CARGO_BIN_EXE_fledge").to_string()
}

pub fn run_fledge(args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin).args(args).output().unwrap()
}

pub fn run_fledge_in(dir: &Path, args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

/// Run fledge with HOME pointed at a fresh tempdir so the invocation sees an
/// empty plugin registry.  The caller owns the `TempDir` — keep it in scope
/// for the duration of the test so the directory is not removed early.
pub fn run_fledge_isolated(args: &[&str], home: &tempfile::TempDir) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .env("HOME", home.path())
        .output()
        .unwrap()
}

/// A hermetic environment for spawning `fledge`.
///
/// The child process gets its own `HOME`, `XDG_CONFIG_HOME` and
/// `FLEDGE_CONFIG_DIR` (all fresh tempdirs), runs non-interactive, has every
/// provider API key stripped from its environment, and has `OLLAMA_HOST`
/// pointed at a closed loopback port. So a command under test can neither read
/// nor write the developer's real `~/.config/fledge/`, nor reach any network
/// endpoint — the AI probes fail fast against a dead local port instead.
///
/// ```ignore
/// let env = TempEnv::new();
/// let out = env.run(&["doctor", "--json"]);
/// assert!(out.status.success());
/// assert!(env.config_dir().join("config.toml").exists());
/// ```
pub struct TempEnv {
    home: tempfile::TempDir,
    config: tempfile::TempDir,
    ollama_host: String,
}

/// Every environment variable that could make a test reach a real endpoint or
/// the developer's own config. Cleared for every `TempEnv` child.
const LEAKY_ENV_VARS: &[&str] = &[
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
    "GITHUB_TOKEN",
    "GH_TOKEN",
];

impl TempEnv {
    pub fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("tempdir for HOME"),
            config: tempfile::tempdir().expect("tempdir for FLEDGE_CONFIG_DIR"),
            ollama_host: format!("http://{}", closed_loopback_addr()),
        }
    }

    pub fn home(&self) -> &Path {
        self.home.path()
    }

    pub fn config_dir(&self) -> &Path {
        self.config.path()
    }

    /// A `fledge` command pre-loaded with the isolated environment. Use when a
    /// test needs to add its own args, env, or working directory.
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(cargo_bin());
        cmd.env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join(".config"))
            .env("FLEDGE_CONFIG_DIR", self.config.path())
            .env("FLEDGE_NON_INTERACTIVE", "1")
            // Nothing is listening here: any AI probe fails immediately
            // instead of contacting a real (possibly cloud) endpoint.
            .env("OLLAMA_HOST", &self.ollama_host);
        for key in LEAKY_ENV_VARS {
            cmd.env_remove(key);
        }
        cmd
    }

    pub fn run(&self, args: &[&str]) -> std::process::Output {
        self.command().args(args).output().unwrap()
    }

    pub fn run_in(&self, dir: &Path, args: &[&str]) -> std::process::Output {
        self.command().args(args).current_dir(dir).output().unwrap()
    }
}

impl Default for TempEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// A loopback address with nothing listening on it.
fn closed_loopback_addr() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind throwaway port");
    let addr = listener.local_addr().expect("throwaway local_addr");
    drop(listener);
    addr
}
