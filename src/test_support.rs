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
