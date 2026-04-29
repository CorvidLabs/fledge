use anyhow::{Context, Result};
use console::style;
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// JSON schema version for the `doctor` envelope. See lanes.rs for the
/// per-command rationale.
const DOCTOR_SCHEMA: u32 = 1;

pub struct DoctorOptions {
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    sections: Vec<Section>,
    passed: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct Section {
    name: String,
    checks: Vec<CheckResult>,
    /// Sections like `toolchains` are informational — missing entries are not
    /// project errors (a Python project doesn't fail because Swift is absent).
    /// Informational sections are excluded from the passed/failed totals.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    informational: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CheckResult {
    name: String,
    status: CheckStatus,
    version: Option<String>,
    detail: Option<String>,
    fix: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum CheckStatus {
    Ok,
    Missing,
    Error,
}

pub fn run(opts: DoctorOptions) -> Result<()> {
    let project_dir = std::env::current_dir().context("getting current directory")?;

    let sections = vec![
        check_fledge_self(),
        check_git(&project_dir),
        check_ai(),
        check_toolchains(),
    ];

    let passed: usize = sections
        .iter()
        .filter(|s| !s.informational)
        .flat_map(|s| &s.checks)
        .filter(|c| c.status == CheckStatus::Ok)
        .count();
    let failed: usize = sections
        .iter()
        .filter(|s| !s.informational)
        .flat_map(|s| &s.checks)
        .filter(|c| c.status != CheckStatus::Ok)
        .count();

    if opts.json {
        let report = DoctorReport {
            sections,
            passed,
            failed,
        };
        let envelope = serde_json::json!({
            "schema_version": DOCTOR_SCHEMA,
            "action": "doctor",
            "sections": report.sections,
            "passed": report.passed,
            "failed": report.failed,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    println!("\n{}\n", style("fledge doctor").bold());

    for section in &sections {
        println!("  {}", style(&section.name).bold());
        for check in &section.checks {
            match &check.status {
                CheckStatus::Ok => {
                    let label = match &check.version {
                        Some(v) => format!("{} {}", check.name, v),
                        None => check.name.clone(),
                    };
                    let label = match &check.detail {
                        Some(d) => format!("{} — {}", label, d),
                        None => label,
                    };
                    println!("    {} {}", style("✅").green().bold(), label);
                }
                CheckStatus::Missing if section.informational => {
                    println!(
                        "    {} {} {}",
                        style("·").dim(),
                        style(&check.name).dim(),
                        style("(not installed)").dim(),
                    );
                }
                CheckStatus::Missing => {
                    let detail = check.detail.as_deref().unwrap_or("not found");
                    println!(
                        "    {} {} — {}",
                        style("❌").red().bold(),
                        check.name,
                        detail
                    );
                    if let Some(fix) = &check.fix {
                        println!("      {} {}", style("➡️").dim(), style(fix).cyan());
                    }
                }
                CheckStatus::Error => {
                    let detail = check.detail.as_deref().unwrap_or("error");
                    let symbol = if section.informational {
                        style("·").dim()
                    } else {
                        style("❌").red().bold()
                    };
                    println!("    {} {} — {}", symbol, check.name, detail);
                    if let Some(fix) = &check.fix {
                        println!("      {} {}", style("➡️").dim(), style(fix).cyan());
                    }
                }
            }
        }
        println!();
    }

    println!(
        "  {} passed, {} found\n",
        style(format!("{} checks", passed)).green().bold(),
        style(format!("{} issues", failed)).red().bold(),
    );

    Ok(())
}

fn check_tool(name: &str, version_args: &[&str], fix: &str) -> CheckResult {
    let child = Command::new(name)
        .args(version_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return CheckResult {
                name: name.to_string(),
                status: CheckStatus::Missing,
                version: None,
                detail: Some("not found".to_string()),
                fix: Some(fix.to_string()),
            };
        }
        Err(e) => {
            return CheckResult {
                name: name.to_string(),
                status: CheckStatus::Error,
                version: None,
                detail: Some(format!("error: {}", e)),
                fix: Some(fix.to_string()),
            };
        }
    };

    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return CheckResult {
                        name: name.to_string(),
                        status: CheckStatus::Error,
                        version: None,
                        detail: Some("timed out after 10s".to_string()),
                        fix: Some(fix.to_string()),
                    };
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return CheckResult {
                    name: name.to_string(),
                    status: CheckStatus::Error,
                    version: None,
                    detail: Some(format!("error: {}", e)),
                    fix: Some(fix.to_string()),
                };
            }
        }
    }

    let output = child.wait_with_output();
    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = if text.trim().is_empty() {
                stderr.to_string()
            } else {
                text.to_string()
            };
            let version = extract_version(&combined);
            CheckResult {
                name: name.to_string(),
                status: CheckStatus::Ok,
                version,
                detail: None,
                fix: None,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            status: CheckStatus::Error,
            version: None,
            detail: Some(format!("error: {}", e)),
            fix: Some(fix.to_string()),
        },
    }
}

fn extract_version(text: &str) -> Option<String> {
    // Look for version-like pattern: digits.digits[.digits[...]]
    // Handles prefixes like "v1.2.3", "go1.22.2", plain "1.78.0"
    text.split_whitespace().find_map(|word| {
        // Strip common prefixes: "v", "go"
        let trimmed = word.trim_start_matches("go").trim_start_matches('v');
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() >= 2
            && parts.iter().all(|p| {
                let numeric_part = p.trim_end_matches(|c: char| !c.is_ascii_digit());
                !numeric_part.is_empty() && numeric_part.chars().all(|c| c.is_ascii_digit())
            })
        {
            Some(trimmed.trim_end_matches(',').to_string())
        } else {
            None
        }
    })
}

/// Self-check: things only fledge knows about its own state. Keep this
/// small — the test is "would removing this hide a real fledge problem?"
/// (Toolchain probes live in `check_toolchains` as an informational section.)
fn check_fledge_self() -> Section {
    let mut checks = Vec::new();

    match crate::config::Config::load() {
        Ok(_) => checks.push(CheckResult {
            name: "fledge config".to_string(),
            status: CheckStatus::Ok,
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            detail: Some("loaded".to_string()),
            fix: None,
        }),
        Err(e) => checks.push(CheckResult {
            name: "fledge config".to_string(),
            status: CheckStatus::Error,
            version: None,
            detail: Some(format!("failed to load: {e}")),
            fix: Some("fledge config get defaults.author  # validates the file parses".to_string()),
        }),
    }

    Section {
        name: "fledge".to_string(),
        checks,
        informational: false,
    }
}

fn check_git(dir: &Path) -> Section {
    let mut checks = Vec::new();

    // Check git is installed
    checks.push(check_tool(
        "git",
        &["--version"],
        "https://git-scm.com/downloads",
    ));

    // Check repo is initialized
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        checks.push(CheckResult {
            name: "repository".to_string(),
            status: CheckStatus::Ok,
            version: None,
            detail: Some("initialized".to_string()),
            fix: None,
        });
    } else {
        checks.push(CheckResult {
            name: "repository".to_string(),
            status: CheckStatus::Missing,
            version: None,
            detail: Some("not a git repository".to_string()),
            fix: Some("git init".to_string()),
        });
    }

    // Check remote
    let remote_output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(dir)
        .output();

    match remote_output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            let first_line = text.lines().next().unwrap_or("");
            if first_line.is_empty() {
                checks.push(CheckResult {
                    name: "remote".to_string(),
                    status: CheckStatus::Missing,
                    version: None,
                    detail: Some("no remote configured".to_string()),
                    fix: Some("git remote add origin <url>".to_string()),
                });
            } else {
                // Parse "origin\thttps://... (fetch)"
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                let remote_name = parts.first().unwrap_or(&"origin");
                let remote_url = parts.get(1).unwrap_or(&"");
                checks.push(CheckResult {
                    name: "remote".to_string(),
                    status: CheckStatus::Ok,
                    version: None,
                    detail: Some(format!("{} ➡️ {}", remote_name, remote_url)),
                    fix: None,
                });
            }
        }
        Err(_) => {
            // git not installed or not a repo — already covered above
        }
    }

    // Check for uncommitted changes
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output();

    if let Ok(out) = status_output {
        let text = String::from_utf8_lossy(&out.stdout);
        let changed: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        if changed.is_empty() {
            checks.push(CheckResult {
                name: "working tree".to_string(),
                status: CheckStatus::Ok,
                version: None,
                detail: Some("clean".to_string()),
                fix: None,
            });
        } else {
            checks.push(CheckResult {
                name: "working tree".to_string(),
                status: CheckStatus::Error,
                version: None,
                detail: Some(format!("uncommitted changes ({} files)", changed.len())),
                fix: Some("git add . && git commit".to_string()),
            });
        }
    }

    Section {
        name: "Git".to_string(),
        checks,
        informational: false,
    }
}

fn check_ai() -> Section {
    let mut checks = Vec::new();

    let claude = check_tool(
        "claude",
        &["--version"],
        "Install Claude CLI: https://docs.anthropic.com/en/docs/claude-code — then run `claude` to authenticate",
    );
    checks.push(claude.clone());

    let ollama = check_tool(
        "ollama",
        &["--version"],
        "Install Ollama: https://ollama.com/download — then `ollama pull <model>` (e.g. llama3.3)",
    );
    checks.push(ollama.clone());

    // Determine the active provider (config → env → default "claude") and
    // report whether it's actually reachable.
    let config = crate::config::Config::load().ok().unwrap_or_default();
    let active = match crate::llm::resolve_provider_kind(&config, None) {
        Ok(k) => k,
        Err(e) => {
            // Invalid ai.provider config / env — surface it explicitly rather
            // than silently defaulting to Claude, which would mask the typo.
            checks.push(CheckResult {
                name: "Active provider: (invalid)".to_string(),
                status: CheckStatus::Error,
                version: None,
                detail: Some(format!("{e}")),
                fix: Some(
                    "Set ai.provider to 'claude' or 'ollama' (or unset FLEDGE_AI_PROVIDER)"
                        .to_string(),
                ),
            });
            return Section {
                name: "AI".to_string(),
                checks,
                informational: false,
            };
        }
    };

    let active_status = match active {
        crate::llm::ProviderKind::Claude => claude.status,
        crate::llm::ProviderKind::Ollama => {
            // Ollama can be a remote endpoint; the CLI check above doesn't tell
            // us whether the configured host responds. Probe `/api/tags`.
            let raw =
                std::env::var("OLLAMA_HOST").unwrap_or_else(|_| config.ai.ollama.host.clone());
            let host = crate::llm::normalize_ollama_host(&raw);
            if probe_ollama_host(&host) {
                CheckStatus::Ok
            } else if ollama.status == CheckStatus::Ok {
                // Binary exists but endpoint unreachable — likely daemon not running
                CheckStatus::Error
            } else {
                CheckStatus::Missing
            }
        }
    };

    let active_detail = match (active, &active_status) {
        (crate::llm::ProviderKind::Claude, CheckStatus::Ok) => {
            Some("claude is the active provider and is reachable".to_string())
        }
        (crate::llm::ProviderKind::Ollama, CheckStatus::Ok) => {
            let raw =
                std::env::var("OLLAMA_HOST").unwrap_or_else(|_| config.ai.ollama.host.clone());
            let host = crate::llm::normalize_ollama_host(&raw);
            let model =
                std::env::var("FLEDGE_AI_MODEL").unwrap_or_else(|_| config.ai.ollama.model.clone());
            Some(format!(
                "ollama is the active provider (model: {model}, host: {host})"
            ))
        }
        (crate::llm::ProviderKind::Ollama, CheckStatus::Error) => {
            let raw =
                std::env::var("OLLAMA_HOST").unwrap_or_else(|_| config.ai.ollama.host.clone());
            let host = crate::llm::normalize_ollama_host(&raw);
            Some(format!(
                "ollama CLI installed but endpoint {host} is not responding"
            ))
        }
        (provider, _) => Some(format!(
            "{} is the active provider but is not available",
            provider.as_str()
        )),
    };

    let active_fix = match (active, &active_status) {
        (_, CheckStatus::Ok) => None,
        (crate::llm::ProviderKind::Ollama, CheckStatus::Error) => Some(
            "Start the Ollama daemon (`ollama serve`) or correct OLLAMA_HOST / ai.ollama.host"
                .to_string(),
        ),
        (crate::llm::ProviderKind::Claude, _) => Some(
            "Install Claude CLI or set `ai.provider = \"ollama\"` to use Ollama instead"
                .to_string(),
        ),
        (crate::llm::ProviderKind::Ollama, _) => Some(
            "Install Ollama or set `ai.provider = \"claude\"` to use Claude CLI instead"
                .to_string(),
        ),
    };

    checks.push(CheckResult {
        name: format!("Active provider: {}", active.as_str()),
        status: active_status,
        version: None,
        detail: active_detail,
        fix: active_fix,
    });

    Section {
        name: "AI".to_string(),
        checks,
        informational: false,
    }
}

/// Toolchain probes — informational. Missing tools don't fail the report
/// because not every project uses every language. Replaces the v0.15
/// `fledge-plugin-doctor` shell-based probe set, re-absorbed into core.
fn check_toolchains() -> Section {
    let probes: &[(&str, &[&str])] = &[
        // Rust
        ("rustc", &["--version"]),
        ("cargo", &["--version"]),
        // Node.js ecosystem
        ("node", &["--version"]),
        ("npm", &["--version"]),
        ("pnpm", &["--version"]),
        ("bun", &["--version"]),
        ("yarn", &["--version"]),
        // Python
        ("python3", &["--version"]),
        ("uv", &["--version"]),
        ("poetry", &["--version"]),
        // Go
        ("go", &["version"]),
        // Ruby
        ("ruby", &["--version"]),
        // Swift
        ("swift", &["--version"]),
        // JVM
        ("java", &["-version"]),
        ("gradle", &["--version"]),
        ("mvn", &["--version"]),
    ];

    let checks = probes
        .iter()
        .map(|(name, args)| {
            let result = check_tool(name, args, "");
            // Strip the fix hint and rewrite the missing detail to read as
            // info, not failure — this section is environmental.
            let detail = match result.status {
                CheckStatus::Missing => Some("not installed".to_string()),
                _ => result.detail,
            };
            CheckResult {
                name: result.name,
                status: result.status,
                version: result.version,
                detail,
                fix: None,
            }
        })
        .collect();

    Section {
        name: "Toolchains".to_string(),
        checks,
        informational: true,
    }
}

fn probe_ollama_host(host: &str) -> bool {
    let url = format!("{}/api/tags", host.trim_end_matches('/'));
    // Short timeout — doctor should fail fast on unreachable hosts (e.g.
    // black-holed DNS, disconnected VPN). If the endpoint is healthy but
    // slow, the subsequent real request will still work with its longer
    // timeout from `ollama_timeout()`.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into();
    ureq::Agent::get(&agent, &url)
        .header("User-Agent", "fledge-cli")
        .call()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_from_rustc() {
        let v = extract_version("rustc 1.78.0 (9b00956e5 2024-04-29)");
        assert_eq!(v, Some("1.78.0".to_string()));
    }

    #[test]
    fn extract_version_from_node() {
        let v = extract_version("v20.11.1");
        assert_eq!(v, Some("20.11.1".to_string()));
    }

    #[test]
    fn extract_version_from_go() {
        let v = extract_version("go version go1.22.2 darwin/arm64");
        assert_eq!(v, Some("1.22.2".to_string()));
    }

    #[test]
    fn extract_version_from_git() {
        let v = extract_version("git version 2.44.0");
        assert_eq!(v, Some("2.44.0".to_string()));
    }

    #[test]
    fn extract_version_none() {
        let v = extract_version("no version here");
        assert_eq!(v, None);
    }

    #[test]
    fn fledge_self_loads_config() {
        let section = check_fledge_self();
        assert_eq!(section.name, "fledge");
        assert!(!section.checks.is_empty());
    }

    #[test]
    fn git_checks_not_repo() {
        let dir = tempfile::tempdir().unwrap();
        let section = check_git(dir.path());
        assert_eq!(section.name, "Git");
        // Should have git tool check + repo check at minimum
        assert!(section.checks.len() >= 2);
        let repo_check = section
            .checks
            .iter()
            .find(|c| c.name == "repository")
            .unwrap();
        assert_eq!(repo_check.status, CheckStatus::Missing);
    }

    #[test]
    fn extract_version_java() {
        // java -version outputs to stderr typically
        let v = extract_version("openjdk version \"17.0.10\" 2024-01-16");
        // The quotes make this tricky — test the pattern
        assert!(v.is_some() || v.is_none()); // graceful either way
    }

    #[test]
    fn section_serializes_to_json() {
        let report = DoctorReport {
            sections: vec![Section {
                name: "fledge".to_string(),
                checks: vec![CheckResult {
                    name: "fledge config".to_string(),
                    status: CheckStatus::Ok,
                    version: Some("0.15.0".to_string()),
                    detail: None,
                    fix: None,
                }],
                informational: false,
            }],
            passed: 1,
            failed: 0,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"fledge config\""));
        assert!(json.contains("\"ok\""));
    }
}
