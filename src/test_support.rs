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
