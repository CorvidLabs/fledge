use anyhow::{Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub(crate) const MAX_EXEC_OUTPUT_SIZE: usize = 10 * 1024 * 1024; // 10 MB per stream

pub(crate) fn handle_exec(
    command: &str,
    cwd: Option<&str>,
    timeout: Option<u64>,
    plugin_dir: &Path,
) -> Result<serde_json::Value> {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let work_dir = match cwd {
        Some(dir) => {
            let resolved = project_root.join(dir);
            let canonical = match resolved.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    return Ok(serde_json::json!({
                        "code": 1,
                        "stdout": "",
                        "stderr": format!("exec cwd '{}' does not exist or is not accessible", dir)
                    }));
                }
            };
            let canonical_root = project_root
                .canonicalize()
                .unwrap_or_else(|_| project_root.clone());
            let canonical_plugin = plugin_dir
                .canonicalize()
                .unwrap_or_else(|_| plugin_dir.to_path_buf());

            if !canonical.starts_with(&canonical_root) && !canonical.starts_with(&canonical_plugin)
            {
                return Ok(serde_json::json!({
                    "code": 1,
                    "stdout": "",
                    "stderr": "exec cwd escapes project and plugin directory"
                }));
            }
            canonical
        }
        None => project_root.clone(),
    };

    const MAX_EXEC_TIMEOUT: u64 = 300;
    let timeout_secs = timeout.unwrap_or(30).min(MAX_EXEC_TIMEOUT);

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/C", command])
        .current_dir(&work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning exec command: {command}"))?;

    #[cfg(not(windows))]
    let mut child = {
        use std::os::unix::process::CommandExt;
        Command::new("sh")
            .args(["-c", command])
            .current_dir(&work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .with_context(|| format!("spawning exec command: {command}"))?
    };

    let result = wait_with_timeout(&mut child, Duration::from_secs(timeout_secs));

    match result {
        Ok(output) => Ok(serde_json::json!({
            "code": output.status.code().unwrap_or(1),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        })),
        Err(_) => {
            kill_child(&mut child);
            Ok(serde_json::json!({
                "code": 124,
                "stdout": "",
                "stderr": format!("command timed out after {}s", timeout_secs)
            }))
        }
    }
}

pub(crate) fn wait_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> Result<std::process::Output, ()> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(out) = child.stdout.take() {
                    out.take(MAX_EXEC_OUTPUT_SIZE as u64)
                        .read_to_end(&mut stdout)
                        .ok();
                }
                if let Some(err) = child.stderr.take() {
                    err.take(MAX_EXEC_OUTPUT_SIZE as u64)
                        .read_to_end(&mut stderr)
                        .ok();
                }
                let status = child
                    .wait()
                    .or_else(|_| std::process::Command::new("true").status())
                    .unwrap_or_else(|_| {
                        // Both wait() and `true` failed — synthesize a failed status
                        #[cfg(unix)]
                        {
                            use std::os::unix::process::ExitStatusExt;
                            std::process::ExitStatus::from_raw(1)
                        }
                        #[cfg(not(unix))]
                        {
                            // On non-unix, run cmd /C exit 1 as last resort
                            std::process::Command::new("cmd")
                                .args(["/C", "exit", "1"])
                                .status()
                                .expect("cannot create exit status")
                        }
                    });
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    return Err(());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(()),
        }
    }
}

pub(crate) fn kill_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as libc::pid_t;
        unsafe {
            libc::killpg(pid, libc::SIGKILL);
        }
    }
    child.kill().ok();
    child.wait().ok();
}
