//! Test-only helpers shared across modules.
//!
//! Lives at the crate root so multiple test modules can serialize on the same
//! `cwd_lock()`. Tests in different modules run on parallel threads, and
//! mutating `std::env::current_dir` is process-global; without a shared
//! mutex, `release::tests` and `lanes::tests` race each other and one
//! observes the other's temp dir mid-flight.

use std::sync::Mutex;

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
