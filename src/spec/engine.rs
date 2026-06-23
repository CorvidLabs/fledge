//! Delegation to the real `specsync` binary.
//!
//! fledge's built-in `spec check` validates spec *structure* (frontmatter,
//! required sections, files exist). The `CorvidLabs/spec-sync` action that runs
//! in CI does far more — it parses each governed source file's public exports
//! and fails when they drift from the spec's Public API tables. Re-implementing
//! that (a 3.7k-LOC, per-language AST layer that ships on its own release
//! cadence) inside fledge would just create a second checker that disagrees with
//! CI over time.
//!
//! So when the `specsync` binary is installed, `fledge spec check` shells out to
//! it — guaranteeing identical results to CI by construction — and renders the
//! findings in fledge's style. When it is absent, the caller falls back to the
//! structural check and hints at `cargo install specsync`.

use anyhow::{bail, Context, Result};
use console::style;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::SPEC_CHECK_SCHEMA;

/// Shape of `specsync check --format json` output. Extra fields are ignored.
#[derive(Debug, Deserialize)]
struct SpecsyncReport {
    passed: bool,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    stale: Vec<serde_json::Value>,
    #[serde(default)]
    specs_checked: usize,
}

/// Locate the `specsync` binary on `PATH`, mirroring `which_fledge_plugin`.
pub(crate) fn find_specsync() -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join("specsync");
        if candidate.is_file() {
            return Some(candidate);
        }
        if cfg!(windows) {
            for ext in &[".exe", ".bat", ".cmd"] {
                let with_ext = dir.join(format!("specsync{ext}"));
                if with_ext.is_file() {
                    return Some(with_ext);
                }
            }
        }
    }
    None
}

/// Best-effort `specsync --version`, for the delegation banner. Never fails the
/// run — a missing version string just yields `None`.
fn specsync_version(bin: &Path) -> Option<String> {
    let output = Command::new(bin).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace().last().map(|s| s.to_string())
}

/// Run `spec check` via the real `specsync` binary when it is installed.
///
/// Returns:
/// - `Ok(None)` — `specsync` is not on `PATH`; the caller should fall back to
///   the built-in structural check.
/// - `Ok(Some(()))` — delegated and the check passed.
/// - `Err(_)` — delegated and the check failed, or the binary could not be run
///   / its output could not be parsed. Propagates to a non-zero exit, matching
///   CI.
pub(crate) fn try_check_via_specsync(root: &Path, strict: bool, json: bool) -> Result<Option<()>> {
    let Some(bin) = find_specsync() else {
        return Ok(None);
    };

    // Mirror the CI action (`specsync check --force` + optional `--strict`):
    // `--force` ignores the local hash cache so the result matches a fresh CI
    // run rather than whatever this machine last recorded.
    let mut args: Vec<String> = vec![
        "check".into(),
        "--force".into(),
        "--format".into(),
        "json".into(),
        "--root".into(),
        root.display().to_string(),
    ];
    if strict {
        args.push("--strict".into());
    }

    let output = Command::new(&bin)
        .args(&args)
        .output()
        .with_context(|| format!("running {}", bin.display()))?;

    // specsync prints its JSON report to stdout for both pass and fail, and
    // exits non-zero on failure. Parse the report rather than trusting the
    // exit code, so we can render findings in fledge's style either way.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: SpecsyncReport = serde_json::from_str(stdout.trim()).map_err(|e| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::anyhow!(
            "specsync ran but its output could not be parsed ({e}).\n{}{}",
            stdout.trim(),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\n{}", stderr.trim())
            }
        )
    })?;

    let version = specsync_version(&bin);
    render(&report, strict, json, version.as_deref())?;

    if report.passed {
        Ok(Some(()))
    } else {
        bail!(
            "spec check failed: {} error(s), {} warning(s)",
            report.errors.len(),
            report.warnings.len()
        )
    }
}

/// Render a specsync report — as fledge's JSON envelope, or fledge-styled text.
fn render(report: &SpecsyncReport, strict: bool, json: bool, version: Option<&str>) -> Result<()> {
    if json {
        let payload = serde_json::json!({
            "schema_version": SPEC_CHECK_SCHEMA,
            "action": "spec_check",
            "engine": "specsync",
            "engine_version": version,
            "passed": report.passed,
            "totals": {
                "checked": report.specs_checked,
                "errors": report.errors.len(),
                "warnings": report.warnings.len(),
            },
            "errors": report.errors,
            "warnings": report.warnings,
            "stale": report.stale,
            "strict": strict,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let banner = match version {
        Some(v) => format!("Delegating to specsync v{v} (matches CI)"),
        None => "Delegating to specsync (matches CI)".to_string(),
    };
    println!("{} {}", style("🔌").cyan(), style(banner).dim());
    println!();

    for error in &report.errors {
        println!("  {} {error}", style("error:").red());
    }
    for warning in &report.warnings {
        println!("  {} {warning}", style("warn:").yellow());
    }
    if !report.errors.is_empty() || !report.warnings.is_empty() {
        println!();
    }

    let summary = format!(
        "{} specs checked, {} {}, {} {}",
        report.specs_checked,
        report.errors.len(),
        if report.errors.len() == 1 {
            "error"
        } else {
            "errors"
        },
        report.warnings.len(),
        if report.warnings.len() == 1 {
            "warning"
        } else {
            "warnings"
        },
    );
    let icon = if report.passed {
        style("✅").green().bold()
    } else {
        style("❌").red().bold()
    };
    println!("  {icon} {summary}");
    if strict && !report.warnings.is_empty() && report.errors.is_empty() && !report.passed {
        println!(
            "  {}",
            style("(warnings treated as errors in strict mode)").dim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_report() {
        let json = r#"{"passed":false,
            "errors":["specs/run/run.spec.md: Spec documents 'foo' but no matching export"],
            "warnings":["specs/work/work.spec.md: Undocumented export 'bar' from src/work.rs"],
            "stale":[{"spec":"specs/x/x.spec.md","reason":"git_drift"}],
            "specs_checked":30}"#;
        let report: SpecsyncReport = serde_json::from_str(json).unwrap();
        assert!(!report.passed);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.stale.len(), 1);
        assert_eq!(report.specs_checked, 30);
    }

    #[test]
    fn parses_minimal_passing_report() {
        // specsync omits empty arrays in some paths; #[serde(default)] must cover them.
        let report: SpecsyncReport =
            serde_json::from_str(r#"{"passed":true,"specs_checked":0}"#).unwrap();
        assert!(report.passed);
        assert!(report.errors.is_empty());
        assert!(report.warnings.is_empty());
        assert!(report.stale.is_empty());
    }

    #[test]
    fn renders_without_panicking() {
        let report = SpecsyncReport {
            passed: false,
            errors: vec!["a: boom".into()],
            warnings: vec!["b: meh".into()],
            stale: vec![],
            specs_checked: 2,
        };
        // Exercises both JSON and text rendering branches.
        render(&report, true, true, Some("4.5.0")).unwrap();
        render(&report, true, false, None).unwrap();
    }
}
