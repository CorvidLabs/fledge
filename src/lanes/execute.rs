use anyhow::{bail, Context, Result};
use console::style;
use std::collections::{BTreeMap, HashSet};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{
    evaluate_when, format_duration, step_description, LaneDef, ParallelItem, Step, TaskDef,
    DEFAULT_RETRY_DELAY_SECS, LANES_RUN_SCHEMA,
};

pub(crate) fn execute_lane(
    lane_name: &str,
    lane: &LaneDef,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    json: bool,
    from_index: Option<usize>,
) -> Result<()> {
    if json {
        return execute_lane_json(lane_name, lane, tasks, project_dir, from_index);
    }

    let desc = lane.description.as_deref().unwrap_or("(no description)");
    println!(
        "{} {} — {}",
        style("▶️ Lane:").cyan().bold(),
        style(lane_name).bold(),
        style(desc).dim()
    );
    if let Some(fi) = from_index {
        println!("  {} starting from step {}", style("⚙").dim(), fi + 1);
    }

    let total_steps = lane.steps.len();
    let mut failures: Vec<String> = Vec::new();
    let lane_start = Instant::now();

    for (i, step) in lane.steps.iter().enumerate() {
        if from_index.is_some_and(|fi| i < fi) {
            println!(
                "  {} Step {} {}",
                style("⏭").dim(),
                i + 1,
                style("(skipped by --from)").dim()
            );
            continue;
        }

        if let Some(when) = step.when() {
            if !evaluate_when(when) {
                println!(
                    "  {} Step {} {} {}",
                    style("⏭").dim(),
                    i + 1,
                    step_description(step),
                    style(format!("(skipped: when '{when}' not met)")).dim()
                );
                continue;
            }
        }

        let retries = step.retries().unwrap_or(0);
        let timeout = step.timeout();
        let retry_delay = step.retry_delay().unwrap_or(DEFAULT_RETRY_DELAY_SECS);
        let step_start = Instant::now();

        let mut last_err = None;
        for attempt in 0..=retries {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(retry_delay));
                println!(
                    "  {} Retry {}/{} for step {}",
                    style("⟳").yellow(),
                    attempt,
                    retries,
                    i + 1
                );
            }
            let deadline = timeout.map(|t| Instant::now() + Duration::from_secs(t));
            let result = execute_step_core(step, tasks, project_dir, false, deadline);
            match result {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        let elapsed = step_start.elapsed();

        if let Some(e) = last_err {
            let step_desc = step_description(step);
            if lane.fail_fast {
                bail!(
                    "Lane '{}' failed at step {} ({}) after {}: {}",
                    lane_name,
                    i + 1,
                    step_desc,
                    format_duration(elapsed),
                    e
                );
            }
            eprintln!(
                "  {} Step {} ({}) failed after {}: {}",
                style("❌").red().bold(),
                i + 1,
                step_desc,
                format_duration(elapsed),
                e
            );
            failures.push(step_desc);
        } else {
            println!(
                "  {} Step {} done {}",
                style("✔").green(),
                i + 1,
                style(format!("({})", format_duration(elapsed))).dim()
            );
        }
    }

    let total_elapsed = lane_start.elapsed();

    if failures.is_empty() {
        println!(
            "{} Lane {} completed ({} steps in {})",
            style("✅").green().bold(),
            style(lane_name).green(),
            total_steps,
            format_duration(total_elapsed)
        );
    } else {
        bail!(
            "Lane '{}' completed with {} failure(s) in {}: {}",
            lane_name,
            failures.len(),
            format_duration(total_elapsed),
            failures.join(", ")
        );
    }

    Ok(())
}

fn execute_lane_json(
    lane_name: &str,
    lane: &LaneDef,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    from_index: Option<usize>,
) -> Result<()> {
    let total_steps = lane.steps.len();
    let mut step_results: Vec<serde_json::Value> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let lane_start = Instant::now();

    for (i, step) in lane.steps.iter().enumerate() {
        let step_desc = step_description(step);

        if from_index.is_some_and(|fi| i < fi) {
            step_results.push(serde_json::json!({
                "step": i + 1,
                "name": step_desc,
                "success": serde_json::Value::Null,
                "duration_ms": serde_json::Value::Null,
                "error": serde_json::Value::Null,
                "skipped": true,
                "reason": "--from",
            }));
            continue;
        }

        if let Some(when) = step.when() {
            if !evaluate_when(when) {
                step_results.push(serde_json::json!({
                    "step": i + 1,
                    "name": step_desc,
                    "success": serde_json::Value::Null,
                    "duration_ms": serde_json::Value::Null,
                    "error": serde_json::Value::Null,
                    "skipped": true,
                    "reason": format!("when '{}' not met", when),
                }));
                continue;
            }
        }

        let retries = step.retries().unwrap_or(0);
        let timeout = step.timeout();
        let retry_delay = step.retry_delay().unwrap_or(DEFAULT_RETRY_DELAY_SECS);
        let step_start = Instant::now();

        let mut attempts = 0u32;
        let mut last_err = None;
        for attempt in 0..=retries {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(retry_delay));
            }
            attempts = attempt + 1;
            let deadline = timeout.map(|t| Instant::now() + Duration::from_secs(t));
            let result = execute_step_core(step, tasks, project_dir, true, deadline);
            match result {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        let elapsed = step_start.elapsed();
        let success = last_err.is_none();
        let error_msg = last_err.map(|e| e.to_string());

        let mut entry = serde_json::json!({
            "step": i + 1,
            "name": step_desc,
            "success": success,
            "duration_ms": elapsed.as_millis() as u64,
            "error": error_msg,
        });
        if attempts > 1 {
            entry["attempts"] = serde_json::json!(attempts);
        }

        step_results.push(entry);

        if !success {
            failures.push(step_desc.clone());
            if lane.fail_fast {
                break;
            }
        }
    }

    let total_elapsed = lane_start.elapsed();
    let success = failures.is_empty();

    let mut output = serde_json::json!({
        "schema_version": LANES_RUN_SCHEMA,
        "lane": lane_name,
        "description": lane.description.as_deref().unwrap_or(""),
        "total_steps": total_steps,
        "success": success,
        "duration_ms": total_elapsed.as_millis() as u64,
        "fail_fast": lane.fail_fast,
        "steps": step_results,
        "failures": failures,
    });
    if let Some(fi) = from_index {
        output["from_step"] = serde_json::json!(fi + 1);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);

    if !success {
        bail!(
            "Lane '{}' completed with {} failure(s)",
            lane_name,
            failures.len()
        );
    }

    Ok(())
}

/// Execute a lane with no console output. Used by internal callers like
/// `release --pre-lane` and the watch task runner where lane prose would
/// pollute the agent's stdout or trigger noisy re-runs. Honors `when`,
/// `timeout`, and `retries` (including the inter-attempt 1s delay, which
/// is silent here — callers in watch mode may see unexplained delays on
/// flaky steps). Deliberately omits `--from`: that flag is user-facing
/// only and silent execution paths always run from step 1.
pub(crate) fn execute_lane_silent(
    lane_name: &str,
    lane: &LaneDef,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
) -> Result<()> {
    let mut failures: Vec<String> = Vec::new();

    for (i, step) in lane.steps.iter().enumerate() {
        if let Some(when) = step.when() {
            if !evaluate_when(when) {
                continue;
            }
        }

        let retries = step.retries().unwrap_or(0);
        let timeout = step.timeout();
        let retry_delay = step.retry_delay().unwrap_or(DEFAULT_RETRY_DELAY_SECS);

        let mut last_err = None;
        for attempt in 0..=retries {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(retry_delay));
            }
            let deadline = timeout.map(|t| Instant::now() + Duration::from_secs(t));
            let result = execute_step_core(step, tasks, project_dir, true, deadline);
            match result {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        if let Some(e) = last_err {
            let step_desc = step_description(step);
            if lane.fail_fast {
                bail!(
                    "Lane '{}' failed at step {} ({}): {}",
                    lane_name,
                    i + 1,
                    step_desc,
                    e
                );
            }
            failures.push(step_desc);
        }
    }

    if !failures.is_empty() {
        bail!(
            "Lane '{}' completed with {} failure(s)",
            lane_name,
            failures.len()
        );
    }

    Ok(())
}

fn execute_step_core(
    step: &Step,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    quiet: bool,
    deadline: Option<Instant>,
) -> Result<()> {
    match step {
        Step::TaskRef(name) | Step::TaskRefFull { task: name, .. } => {
            execute_task_with_deps(name, tasks, project_dir, quiet, deadline)
        }
        Step::Inline { run: cmd, .. } => execute_inline(cmd, project_dir, quiet, deadline),
        Step::Parallel { parallel, .. } => {
            execute_parallel(parallel, tasks, project_dir, quiet, deadline)
        }
    }
}

pub(crate) fn execute_task_with_deps(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    quiet: bool,
    deadline: Option<Instant>,
) -> Result<()> {
    if let Some(d) = deadline {
        if Instant::now() >= d {
            bail!("step timed out");
        }
    }
    let mut visited = HashSet::new();
    execute_task_recursive(name, tasks, project_dir, &mut visited, quiet, deadline)
}

fn execute_task_recursive(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    visited: &mut HashSet<String>,
    quiet: bool,
    deadline: Option<Instant>,
) -> Result<()> {
    if !visited.insert(name.to_string()) {
        bail!(
            "Circular dependency detected: task '{}' depends on itself (chain: {})",
            name,
            visited.iter().cloned().collect::<Vec<_>>().join(" → ")
        );
    }

    let task = tasks
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", name))?;

    for dep in task.deps() {
        execute_task_recursive(dep, tasks, project_dir, visited, quiet, deadline)?;
    }

    execute_single_task(name, task, project_dir, quiet, deadline)
}

fn execute_single_task(
    name: &str,
    task: &TaskDef,
    project_dir: &Path,
    quiet: bool,
    deadline: Option<Instant>,
) -> Result<()> {
    if !quiet {
        println!(
            "  {} {}",
            style("▶️").cyan().bold(),
            style(format!("Running task: {name}")).bold()
        );
    }

    let cmd_str = task.cmd();
    let work_dir = match task.dir() {
        Some(d) => project_dir.join(d),
        None => project_dir.to_path_buf(),
    };

    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    let mut command = Command::new(shell);
    command.arg(flag).arg(cmd_str).current_dir(&work_dir);

    for (key, value) in task.env() {
        command.env(key, value);
    }

    if quiet {
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
    }

    let status = run_command_with_deadline(&mut command, deadline)
        .with_context(|| format!("running task '{name}'"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Task '{}' failed with exit code {}", name, code);
    }

    Ok(())
}

fn execute_inline(
    cmd: &str,
    project_dir: &Path,
    quiet: bool,
    deadline: Option<Instant>,
) -> Result<()> {
    if !quiet {
        println!(
            "  {} {}",
            style("▶️").cyan().bold(),
            style(format!("Running: {cmd}")).bold()
        );
    }

    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    let mut command = Command::new(shell);
    command.arg(flag).arg(cmd).current_dir(project_dir);
    if quiet {
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
    }

    let status = run_command_with_deadline(&mut command, deadline)
        .with_context(|| format!("running inline command: {cmd}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Inline command failed with exit code {}: {}", code, cmd);
    }

    Ok(())
}

fn execute_parallel(
    items: &[ParallelItem],
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    quiet: bool,
    deadline: Option<Instant>,
) -> Result<()> {
    let names_display: Vec<String> = items
        .iter()
        .map(|item| match item {
            ParallelItem::TaskRef(name) => name.clone(),
            ParallelItem::Inline { run: cmd } => cmd.clone(),
        })
        .collect();
    if !quiet {
        println!(
            "  {} {}",
            style("▶️").cyan().bold(),
            style(format!("Running parallel: {}", names_display.join(", "))).bold()
        );
    }

    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|s| {
        let mut handles = Vec::new();

        for item in items {
            let errors = Arc::clone(&errors);
            let handle = s.spawn(move || {
                let result = match item {
                    ParallelItem::TaskRef(name) => {
                        execute_task_with_deps(name, tasks, project_dir, quiet, deadline)
                    }
                    ParallelItem::Inline { run: cmd } => {
                        execute_inline(cmd, project_dir, quiet, deadline)
                    }
                };
                if let Err(e) = result {
                    let label = match item {
                        ParallelItem::TaskRef(name) => name.clone(),
                        ParallelItem::Inline { run: cmd } => cmd.clone(),
                    };
                    if let Ok(mut errs) = errors.lock() {
                        errs.push(format!("{}: {}", label, e));
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            if let Err(panic_val) = handle.join() {
                let panic_msg = panic_val
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic_val.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("unknown panic");
                if let Ok(mut errs) = errors.lock() {
                    errs.push(format!("thread panic: {}", panic_msg));
                }
            }
        }
    });

    let errs = errors.lock().unwrap_or_else(|e| e.into_inner());
    if !errs.is_empty() {
        bail!("Parallel step failed:\n  {}", errs.join("\n  "));
    }

    Ok(())
}

fn run_command_with_deadline(
    command: &mut Command,
    deadline: Option<Instant>,
) -> Result<std::process::ExitStatus> {
    match deadline {
        Some(d) => {
            if Instant::now() >= d {
                bail!("step timed out");
            }

            // Reap entire process tree on timeout. Without this, a shell
            // command like `sh -c "echo a && sleep 30"` would leave
            // grandchildren running after the direct child is killed.
            //
            // - Unix: spawn in a fresh process group, signal it with
            //   `killpg(SIGKILL)` on timeout.
            // - Windows: assign the child to a Job Object configured
            //   with KILL_ON_JOB_CLOSE; on timeout, `TerminateJobObject`
            //   kills every process in the job tree.
            #[cfg(unix)]
            command.process_group(0);
            let mut child = command.spawn().context("spawning command")?;
            #[cfg(unix)]
            let pgid = child.id() as libc::pid_t;
            #[cfg(windows)]
            let job = setup_windows_job(&child);

            loop {
                match child.try_wait().context("waiting for command")? {
                    Some(status) => {
                        #[cfg(windows)]
                        if let Some(h) = job {
                            // SAFETY: handle returned by CreateJobObjectW; closing it
                            // after the child has exited cannot affect a living tree.
                            unsafe {
                                windows_sys::Win32::Foundation::CloseHandle(h);
                            }
                        }
                        return Ok(status);
                    }
                    None => {
                        if Instant::now() >= d {
                            #[cfg(unix)]
                            // SAFETY: killpg(pgid, SIGKILL) — pgid is the child's own
                            // pgid (set via process_group(0) above) and SIGKILL has no
                            // signal handler to invoke.
                            unsafe {
                                libc::killpg(pgid, libc::SIGKILL);
                            }
                            #[cfg(windows)]
                            if let Some(h) = job {
                                // SAFETY: TerminateJobObject + CloseHandle on a job we
                                // own. Exit code 1 stands for the synthetic kill.
                                unsafe {
                                    windows_sys::Win32::System::JobObjects::TerminateJobObject(
                                        h, 1,
                                    );
                                    windows_sys::Win32::Foundation::CloseHandle(h);
                                }
                            } else {
                                let _ = child.kill();
                            }
                            let _ = child.wait();
                            bail!("step timed out");
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        }
        None => Ok(command.status()?),
    }
}

/// Wrap the spawned child in a Job Object so that `TerminateJobObject`
/// reaps the entire process tree on timeout. The race window between
/// `spawn` and `AssignProcessToJobObject` is small for typical shell
/// invocations — a child rarely spawns grandchildren before the assign
/// call lands a few microseconds later. If setup fails (rare; would
/// require resource exhaustion), returns `None` and the timeout path
/// falls back to `child.kill()` which only signals the direct child.
#[cfg(windows)]
fn setup_windows_job(
    child: &std::process::Child,
) -> Option<windows_sys::Win32::Foundation::HANDLE> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    // SAFETY: CreateJobObjectW with null name returns a fresh unnamed job
    // handle or null on failure.
    let job: HANDLE = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return None;
    }

    // SAFETY: zero-initialized struct is valid for JOBOBJECT_EXTENDED_LIMIT_INFORMATION;
    // we only set LimitFlags. Pointer is to a stack value valid for the call duration.
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let ok = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(job) };
        return None;
    }

    // SAFETY: child's raw handle is valid for the duration of the Child borrow;
    // AssignProcessToJobObject does not retain the handle past the call.
    let assigned = unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) };
    if assigned == 0 {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(job) };
        return None;
    }

    Some(job)
}
