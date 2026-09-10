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
/// What it guarantees, precisely:
///
/// * **Config isolation.** `HOME`, `XDG_CONFIG_HOME` and `FLEDGE_CONFIG_DIR`
///   are fresh tempdirs, so the child can neither read nor write the
///   developer's real `~/.config/fledge/`.
/// * **Credential isolation.** Every provider API key and GitHub token in
///   [`LEAKY_ENV_VARS`] is removed from the child's environment.
/// * **Git identity isolation.** Author/committer identity is pinned to a test
///   value and the global/system gitconfig is ignored, so commit-producing
///   commands work on a machine with no git identity and never read the
///   developer's `~/.gitconfig`.
/// * **Non-interactive.** No prompt can block the test.
/// * **Endpoint isolation, for the endpoints fledge resolves itself.**
///   `OLLAMA_HOST`, the GitHub REST base and the `github.com/<owner>/<repo>.git`
///   git base all point at a *closed* loopback port, so an AI probe, a
///   `templates search`, or a remote-template `git clone` fails immediately
///   against a dead local port instead of reaching a real endpoint. Point them
///   somewhere useful with [`TempEnv::with_github_api_base`] /
///   [`TempEnv::with_github_remote_base`].
///
/// What it does **not** guarantee — do not assume otherwise:
///
/// * It is not a network sandbox. It redirects the endpoints fledge builds
///   from `github::api_base()` / `github::remote_base()` and the AI host. A
///   command handed an explicit URL (`plugins install <url>`, `lanes import
///   <url>`) or a plugin subprocess of its own still goes wherever it is told.
///   Tests for those paths must supply a local URL themselves.
/// * The GitHub redirection rides on the `FLEDGE_TEST_GITHUB_*` variables,
///   which the binary only honours in **debug builds** (see
///   `github::test_endpoint_override`). CI and `cargo test` use the debug
///   profile; under `cargo test --release` the hook is compiled out, and tests
///   that depend on it skip themselves rather than reach the network.
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
    github_api_base: String,
    github_remote_base: String,
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
        let dead = format!("http://{}", closed_loopback_addr());
        Self {
            home: tempfile::tempdir().expect("tempdir for HOME"),
            config: tempfile::tempdir().expect("tempdir for FLEDGE_CONFIG_DIR"),
            ollama_host: dead.clone(),
            github_api_base: dead.clone(),
            github_remote_base: dead,
        }
    }

    /// Point the child's GitHub REST base at `base` — a [`MockHttp`] URL, or
    /// any other loopback address. Non-loopback values are refused by the
    /// binary, so this cannot be used to reach the real API.
    pub fn with_github_api_base(mut self, base: impl Into<String>) -> Self {
        self.github_api_base = base.into();
        self
    }

    /// Point the child's git remote base at `base`: a loopback URL, or a local
    /// directory holding `<owner>/<repo>.git` bare repos, which then stands in
    /// for github.com for `templates init <owner>/<repo>` and publish pushes.
    pub fn with_github_remote_base(mut self, base: impl Into<String>) -> Self {
        self.github_remote_base = base.into();
        self
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
        let absent_gitconfig = self.home.path().join("nonexistent-gitconfig");
        let mut cmd = Command::new(cargo_bin());
        cmd.env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join(".config"))
            // Where the remote-template clone cache lands. `dirs::cache_dir()`
            // reads this on Linux; on macOS it derives from HOME. Windows uses
            // a known-folder API that no environment variable can redirect.
            .env("XDG_CACHE_HOME", self.home.path().join(".cache"))
            .env("FLEDGE_CONFIG_DIR", self.config.path())
            .env("FLEDGE_NON_INTERACTIVE", "1")
            // Nothing is listening here: any AI probe fails immediately
            // instead of contacting a real (possibly cloud) endpoint.
            .env("OLLAMA_HOST", &self.ollama_host)
            // Same for GitHub. The spawned binary can't see the in-process
            // `GithubBaseGuard`, so the redirection has to travel as env.
            // Default is a dead loopback port: a stray `templates search` or
            // remote-template clone fails fast instead of hitting github.com.
            .env("FLEDGE_TEST_GITHUB_API_BASE", &self.github_api_base)
            .env("FLEDGE_TEST_GITHUB_REMOTE_BASE", &self.github_remote_base)
            // git works with no developer identity and never reads ~/.gitconfig.
            .env("GIT_AUTHOR_NAME", "fledge test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "fledge test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .env("GIT_CONFIG_GLOBAL", &absent_gitconfig)
            .env("GIT_CONFIG_NOSYSTEM", "1");
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

/// Whether the binary under test honours the `FLEDGE_TEST_GITHUB_*`
/// redirection. It is a debug-build-only hook, so a test that needs it must
/// skip itself under `cargo test --release` rather than fall through to the
/// real github.com.
pub fn github_redirection_supported() -> bool {
    // The test binary and the `fledge` binary it spawns are built by the same
    // cargo invocation, so they share a profile.
    cfg!(debug_assertions)
}

/// A minimal loopback HTTP server for integration tests.
///
/// `src/test_support.rs::MockHttpServer` is the richer in-process version, but
/// integration tests are a separate crate and fledge has no library target, so
/// they cannot reach it. This serves one canned JSON body to every request and
/// records what it was asked for — enough to prove a spawned `fledge` really
/// talked to it and not to api.github.com.
pub struct MockHttp {
    addr: std::net::SocketAddr,
    requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl MockHttp {
    /// Bind an ephemeral loopback port and answer every request with
    /// `200 application/json` and `body`.
    pub fn start(body: &str) -> Self {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock http");
        let addr = listener.local_addr().expect("mock http local_addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let thread_requests = Arc::clone(&requests);
        let thread_shutdown = Arc::clone(&shutdown);
        let body = body.to_string();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if thread_shutdown.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { break };
                // Sequential: every response closes its connection, so a
                // client never waits on a second one. No handler threads to
                // leak.
                serve_one(stream, &body, &thread_requests);
            }
        });

        Self {
            addr,
            requests,
            shutdown,
            handle: Some(handle),
        }
    }

    /// Base URL, e.g. `http://127.0.0.1:54321` (no trailing slash).
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Every request line received so far, e.g. `GET /search/repositories?q=…`.
    pub fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Drop for MockHttp {
    fn drop(&mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        // Wake the blocking `accept` so the loop sees the flag.
        let _ = std::net::TcpStream::connect(self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn serve_one(
    mut stream: std::net::TcpStream,
    body: &str,
    requests: &std::sync::Arc<std::sync::Mutex<Vec<String>>>,
) {
    use std::io::{BufRead, BufReader, Write};

    let Ok(peek) = stream.try_clone() else { return };
    let mut reader = BufReader::new(peek);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.trim().is_empty() {
        return; // shutdown probe or dead socket
    }
    // Drain headers so the client isn't writing into a closed socket.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) if line.trim_end_matches(['\r', '\n']).is_empty() => break,
            Ok(_) => {}
            Err(_) => return,
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    requests
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(format!("{method} {target}"));

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// A loopback address with nothing listening on it.
fn closed_loopback_addr() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind throwaway port");
    let addr = listener.local_addr().expect("throwaway local_addr");
    drop(listener);
    addr
}
