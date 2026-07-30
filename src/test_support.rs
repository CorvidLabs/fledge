//! Test-only helpers shared across modules.
//!
//! Lives at the crate root so multiple test modules can serialize on the same
//! `cwd_lock()`. Tests in different modules run on parallel threads, and
//! mutating `std::env::current_dir` is process-global; without a shared
//! mutex, `release::tests` and `lanes::tests` race each other and one
//! observes the other's temp dir mid-flight.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the process-wide cwd mutex. Hold the returned guard for the
/// duration of any block that calls `std::env::set_current_dir`.
pub(crate) fn cwd_lock() -> std::sync::MutexGuard<'static, ()> {
    // Recover from a poisoned lock — a previous test panicked while holding
    // it. The protected state is just "who's currently mutating cwd," and
    // a panic doesn't corrupt that, so it's safe to take over.
    CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The non-interactive flag is a process-wide `AtomicBool` in `crate::utils`.
/// Tests that flip it must serialize on the same mutex so they don't race.
static NON_INTERACTIVE_LOCK: Mutex<()> = Mutex::new(());

/// RAII guard: sets `crate::utils::set_non_interactive(value)` for the
/// duration, then restores the previous value on drop. Holds the
/// process-wide `NON_INTERACTIVE_LOCK` so concurrent tests don't observe
/// each other's transient state.
pub(crate) struct NonInteractiveGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev: bool,
}

impl NonInteractiveGuard {
    pub(crate) fn new(set_to: bool) -> Self {
        let lock = NON_INTERACTIVE_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let prev = crate::utils::is_non_interactive();
        crate::utils::set_non_interactive(set_to);
        Self { _lock: lock, prev }
    }
}

impl Drop for NonInteractiveGuard {
    fn drop(&mut self) {
        crate::utils::set_non_interactive(self.prev);
    }
}

/// Environment variables are process-global; cargo runs unit tests on parallel
/// threads. Tests in different modules that read or mutate the same variables
/// (`FLEDGE_AI_PROVIDER`, `OLLAMA_HOST`, `FLEDGE_CONFIG_DIR`, …) must serialize
/// on one lock. Before this, `ai.rs` and `llm.rs` each defined their own
/// private `static LOCK`, so an `ai` test and an `llm` test could run at the
/// same time and clobber each other's env — an intermittent, order-dependent
/// failure. One lock for the whole test binary removes that race.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the process-wide environment-variable mutex. Hold the returned guard
/// for the duration of any test that reads or mutates environment variables.
pub(crate) fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    // Recover from a poisoned lock (a panic in another env test). The protected
    // state is just "who's currently touching env," which a panic can't corrupt
    // — every guard below restores what it changed.
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// RAII guard: sets a single environment variable to `value` (or removes it
/// when `None`) for the test's duration, restoring the previous value on drop —
/// even on panic. Hold [`env_lock`] alongside it, since env is process-global.
pub(crate) struct EnvVarGuard {
    key: String,
    previous: Option<String>,
}

impl EnvVarGuard {
    pub(crate) fn set(key: &str, value: Option<&str>) -> Self {
        let previous = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        Self {
            key: key.to_string(),
            previous,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

/// RAII guard pointing `FLEDGE_CONFIG_DIR` at a fresh, empty tempdir for the
/// test's duration, restoring the previous value (or unsetting it) on drop —
/// even on panic. Keeps config-reading tests off the developer's real
/// `~/.config/fledge/config.toml`. Hold [`env_lock`] alongside it.
pub(crate) struct ConfigDirGuard {
    tmp: tempfile::TempDir,
    previous: Option<String>,
}

impl ConfigDirGuard {
    pub(crate) fn new() -> Self {
        let previous = std::env::var("FLEDGE_CONFIG_DIR").ok();
        let tmp = tempfile::tempdir().expect("create tempdir for FLEDGE_CONFIG_DIR");
        std::env::set_var("FLEDGE_CONFIG_DIR", tmp.path());
        Self { tmp, previous }
    }

    /// The isolated config directory (empty until the test writes into it).
    #[allow(dead_code)]
    pub(crate) fn path(&self) -> &std::path::Path {
        self.tmp.path()
    }
}

impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var("FLEDGE_CONFIG_DIR", v),
            None => std::env::remove_var("FLEDGE_CONFIG_DIR"),
        }
    }
}

/// Run `f` with the process current directory set to `dir`, serialized on the
/// shared [`cwd_lock`] and restoring the previous directory afterward — even on
/// panic. Use to drive production helpers that shell out to `git` (or any tool)
/// in the current directory. Because the CWD is process-global, this holds
/// `cwd_lock` for the whole closure; keep `f` short.
pub(crate) fn with_cwd<F: FnOnce() -> R, R>(dir: &std::path::Path, f: F) -> R {
    let _guard = cwd_lock();
    let saved = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let _ = std::env::set_current_dir(saved);
    match result {
        Ok(r) => r,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// A throwaway git repository in a tempdir, for exercising the git-subprocess
/// helpers against a real `git` — the same approach `release::tests` already
/// uses (real git is more faithful than any canned-stdout double). Seed it with
/// the builder methods, then drive a CWD-bound production helper via
/// [`TestRepo::run_in`]. The repo is deleted when the `TestRepo` drops.
pub(crate) struct TestRepo {
    dir: tempfile::TempDir,
}

impl TestRepo {
    /// Initialize a repo (`git init` + a committer identity) in a fresh tempdir.
    pub(crate) fn init() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().expect("create tempdir for TestRepo"),
        };
        repo.git(&["init"]);
        repo.git(&["config", "user.email", "test@test.com"]);
        repo.git(&["config", "user.name", "Test"]);
        repo
    }

    /// The repository's working-directory path.
    #[allow(dead_code)]
    pub(crate) fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    /// Run a git command in the repo, returning its `Output`. Panics only if
    /// `git` can't be spawned; the exit status is left for the caller to
    /// inspect (setup steps like `symbolic-ref` may legitimately be checked).
    pub(crate) fn git(&self, args: &[&str]) -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(self.dir.path())
            .output()
            .expect("spawn git")
    }

    /// Write `content` to `name` and commit it as "add {name}". Returns `&self`
    /// for chaining.
    pub(crate) fn commit_file(&self, name: &str, content: &str) -> &Self {
        std::fs::write(self.dir.path().join(name), content).expect("write test file");
        self.git(&["add", name]);
        self.git(&["commit", "-m", &format!("add {name}")]);
        self
    }

    /// Run `f` with the process CWD set to this repo (see [`with_cwd`]).
    pub(crate) fn run_in<F: FnOnce() -> R, R>(&self, f: F) -> R {
        with_cwd(self.dir.path(), f)
    }
}

// ── HTTP mocking harness ──────────────────────────────────────────────────
//
// Every network path in fledge (LLM completions, the GitHub REST API, remote
// template fetch) speaks plain HTTP through `ureq`, which is blocking. A
// blocking, dependency-free loopback server is therefore a better fit than an
// async mock crate: no tokio/wiremock in the dependency tree, and the
// production code under test runs unmodified on the calling thread.
//
// Usage:
//
// ```ignore
// let server = MockHttpServer::start();
// server.on("GET", "/user", MockResponse::json(200, r#"{"login":"octo"}"#));
// let _base = GithubBaseGuard::api(&server.url());
// assert_eq!(get_authenticated_user("tok").unwrap(), "octo");
// assert_eq!(server.requests()[0].header("authorization"), Some("Bearer tok"));
// ```
//
// The server binds `127.0.0.1:0` (loopback only — never the real network) and
// shuts down when the `MockHttpServer` drops.

/// A canned HTTP response served by [`MockHttpServer`].
#[derive(Clone)]
pub(crate) struct MockResponse {
    pub(crate) status: u16,
    pub(crate) body: String,
    pub(crate) content_type: String,
}

impl MockResponse {
    /// `application/json` response with a raw body string.
    pub(crate) fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            content_type: "application/json".to_string(),
        }
    }

    /// `text/plain` response — for exercising "server returned junk" paths.
    pub(crate) fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            content_type: "text/plain".to_string(),
        }
    }

    /// Status-only response with an empty JSON object body.
    pub(crate) fn empty(status: u16) -> Self {
        Self::json(status, "{}")
    }
}

/// One request the [`MockHttpServer`] received, captured for assertions.
#[derive(Clone, Debug)]
pub(crate) struct RecordedRequest {
    pub(crate) method: String,
    /// Request path, query string stripped.
    pub(crate) path: String,
    /// Raw query string (no leading `?`), empty when absent.
    pub(crate) query: String,
    /// Header names lowercased, in the order received.
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: String,
}

impl RecordedRequest {
    /// Case-insensitive header lookup.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        let want = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| *k == want)
            .map(|(_, v)| v.as_str())
    }

    /// Parse the request body as JSON (panics when it is not valid JSON).
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|e| panic!("request body was not JSON ({e}): {}", self.body))
    }
}

struct MockState {
    routes: HashMap<(String, String), MockResponse>,
    fallback: MockResponse,
    requests: Vec<RecordedRequest>,
}

/// A loopback HTTP server for testing code that makes real HTTP calls.
/// Register routes with [`MockHttpServer::on`], point the code under test at
/// [`MockHttpServer::url`], then assert on [`MockHttpServer::requests`].
pub(crate) struct MockHttpServer {
    addr: SocketAddr,
    state: Arc<Mutex<MockState>>,
    shutdown: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl MockHttpServer {
    /// Bind an ephemeral loopback port and start serving.
    pub(crate) fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock http server");
        let addr = listener.local_addr().expect("mock server local_addr");
        let state = Arc::new(Mutex::new(MockState {
            routes: HashMap::new(),
            fallback: MockResponse::json(404, r#"{"message":"no mock route registered"}"#),
            requests: Vec::new(),
        }));
        let shutdown = Arc::new(AtomicBool::new(false));

        let thread_state = Arc::clone(&state);
        let thread_shutdown = Arc::clone(&shutdown);
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if thread_shutdown.load(Ordering::SeqCst) {
                    break;
                }
                match stream {
                    // One thread per connection: `ureq` may hold a pooled
                    // connection open while opening another, so serving
                    // sequentially could deadlock a multi-request test.
                    Ok(s) => {
                        let conn_state = Arc::clone(&thread_state);
                        std::thread::spawn(move || handle_connection(s, conn_state));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            addr,
            state,
            shutdown,
            handle: Some(handle),
        }
    }

    /// Base URL of the server, e.g. `http://127.0.0.1:54321` (no trailing slash).
    pub(crate) fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Register the response for `METHOD path`. Re-registering replaces.
    pub(crate) fn on(&self, method: &str, path: &str, response: MockResponse) -> &Self {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .routes
            .insert((method.to_ascii_uppercase(), path.to_string()), response);
        self
    }

    /// Response for requests that match no registered route (default: 404).
    pub(crate) fn fallback(&self, response: MockResponse) -> &Self {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .fallback = response;
        self
    }

    /// Every request received so far, in order.
    pub(crate) fn requests(&self) -> Vec<RecordedRequest> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .requests
            .clone()
    }

    /// The first request matching `METHOD path`, or `None`.
    pub(crate) fn request(&self, method: &str, path: &str) -> Option<RecordedRequest> {
        let want = method.to_ascii_uppercase();
        self.requests()
            .into_iter()
            .find(|r| r.method == want && r.path == path)
    }
}

impl Drop for MockHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Wake the blocking `accept` so the loop observes the shutdown flag.
        let _ = TcpStream::connect(self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn handle_connection(stream: TcpStream, state: Arc<Mutex<MockState>>) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.trim().is_empty() {
        return; // shutdown probe or dead socket
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target, String::new()),
    };

    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']);
                if trimmed.is_empty() {
                    break;
                }
                if let Some((k, v)) = trimmed.split_once(':') {
                    headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
                }
            }
            Err(_) => break,
        }
    }

    let content_length: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        return;
    }

    let response = {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        guard.requests.push(RecordedRequest {
            method: method.to_ascii_uppercase(),
            path: path.clone(),
            query,
            headers,
            body: String::from_utf8_lossy(&body).to_string(),
        });
        guard
            .routes
            .get(&(method.to_ascii_uppercase(), path))
            .cloned()
            .unwrap_or_else(|| guard.fallback.clone())
    };

    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let mut stream = stream;
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(response.body.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// A loopback port with nothing listening on it — for exercising
/// connection-refused paths without waiting on a real network timeout.
pub(crate) fn dead_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind throwaway port");
    let addr = listener.local_addr().expect("throwaway local_addr");
    drop(listener);
    format!("http://{}", addr)
}

// ── GitHub endpoint redirection (test builds only) ────────────────────────
//
// `github.rs` and `publish.rs` hardcode `https://api.github.com` and
// `https://github.com`. These thread-local overrides let a test point those
// two bases at a `MockHttpServer` / a local bare repo, so the *real*
// production call paths (including the shared `run_publish` orchestration)
// run end to end without touching the network. They are `#[cfg(test)]`-only:
// the release build keeps plain constants. Thread-local (not a global) so
// tests running in parallel cannot see each other's redirection.

thread_local! {
    static GITHUB_API_BASE: RefCell<Option<String>> = const { RefCell::new(None) };
    static GITHUB_REMOTE_BASE: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn github_api_base_override() -> Option<String> {
    GITHUB_API_BASE.with(|c| c.borrow().clone())
}

pub(crate) fn github_remote_base_override() -> Option<String> {
    GITHUB_REMOTE_BASE.with(|c| c.borrow().clone())
}

/// RAII guard redirecting the GitHub REST base URL (and optionally the git
/// remote base) for the current thread, restoring the previous values on drop.
pub(crate) struct GithubBaseGuard {
    prev_api: Option<String>,
    prev_remote: Option<String>,
}

impl GithubBaseGuard {
    /// Redirect the REST API base only (e.g. to a [`MockHttpServer`] URL).
    pub(crate) fn api(base: &str) -> Self {
        let prev_api = GITHUB_API_BASE.with(|c| c.replace(Some(base.to_string())));
        let prev_remote = GITHUB_REMOTE_BASE.with(|c| c.borrow().clone());
        Self {
            prev_api,
            prev_remote,
        }
    }

    /// Redirect both the REST API base and the git remote base. `remote_base`
    /// is joined as `{remote_base}/{owner}/{repo}.git`, so a directory
    /// containing `<owner>/<repo>.git` bare repos stands in for github.com.
    pub(crate) fn api_and_remote(api_base: &str, remote_base: &str) -> Self {
        let guard = Self::api(api_base);
        GITHUB_REMOTE_BASE.with(|c| c.replace(Some(remote_base.to_string())));
        guard
    }
}

impl Drop for GithubBaseGuard {
    fn drop(&mut self) {
        GITHUB_API_BASE.with(|c| c.replace(self.prev_api.clone()));
        GITHUB_REMOTE_BASE.with(|c| c.replace(self.prev_remote.clone()));
    }
}

/// Point git's author/committer identity and config lookup at values the test
/// controls, so commit-producing helpers work on a machine with no global git
/// identity and never read the developer's `~/.gitconfig`. Hold [`env_lock`]
/// alongside it — the variables are process-global.
pub(crate) struct GitIdentityGuard {
    _vars: Vec<EnvVarGuard>,
}

impl GitIdentityGuard {
    pub(crate) fn new(no_global_config_at: &std::path::Path) -> Self {
        let absent = no_global_config_at.join("nonexistent-gitconfig");
        let absent = absent.to_string_lossy().to_string();
        Self {
            _vars: vec![
                EnvVarGuard::set("GIT_AUTHOR_NAME", Some("fledge test")),
                EnvVarGuard::set("GIT_AUTHOR_EMAIL", Some("test@example.invalid")),
                EnvVarGuard::set("GIT_COMMITTER_NAME", Some("fledge test")),
                EnvVarGuard::set("GIT_COMMITTER_EMAIL", Some("test@example.invalid")),
                EnvVarGuard::set("GIT_CONFIG_GLOBAL", Some(&absent)),
                EnvVarGuard::set("GIT_CONFIG_NOSYSTEM", Some("1")),
            ],
        }
    }
}

/// The canned result a [`StubLlmProvider`] yields from `invoke`.
pub(crate) enum StubOutcome {
    Ok(String),
    Err(String),
}

/// A canned [`LlmProvider`](crate::llm::LlmProvider) for exercising code that
/// fans out over providers (e.g. `review::run_panel`) with no network I/O. It
/// returns a preset outcome and reports a fixed provider kind / model, so tests
/// can assert ordering, per-slot error isolation, and metadata capture without
/// a live endpoint. Shared so the `review` / `ask` / `ai` test modules can all
/// reuse the same double.
pub(crate) struct StubLlmProvider {
    kind: crate::llm::ProviderKind,
    model: Option<String>,
    outcome: StubOutcome,
}

impl StubLlmProvider {
    /// A provider whose `invoke` succeeds with `response`.
    pub(crate) fn ok(kind: crate::llm::ProviderKind, model: Option<&str>, response: &str) -> Self {
        Self {
            kind,
            model: model.map(str::to_string),
            outcome: StubOutcome::Ok(response.to_string()),
        }
    }

    /// A provider whose `invoke` fails with `message` (as an `anyhow` error).
    pub(crate) fn err(kind: crate::llm::ProviderKind, model: Option<&str>, message: &str) -> Self {
        Self {
            kind,
            model: model.map(str::to_string),
            outcome: StubOutcome::Err(message.to_string()),
        }
    }
}

impl crate::llm::LlmProvider for StubLlmProvider {
    fn invoke(&self, _prompt: &str) -> anyhow::Result<String> {
        match &self.outcome {
            StubOutcome::Ok(s) => Ok(s.clone()),
            StubOutcome::Err(e) => anyhow::bail!("{e}"),
        }
    }

    fn kind(&self) -> crate::llm::ProviderKind {
        self.kind
    }

    fn model_name(&self) -> Option<&str> {
        self.model.as_deref()
    }
}
