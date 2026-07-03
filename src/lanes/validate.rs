use anyhow::{bail, Context, Result};
use console::style;
use std::collections::HashSet;
use std::path::Path;

use super::{FledgeFileWithLanes, ParallelItem, Step};

#[derive(Default, serde::Serialize)]
pub(crate) struct LaneValidationReport {
    pub(super) path: String,
    pub(super) lane_count: usize,
    pub(super) errors: Vec<String>,
    pub(super) warnings: Vec<String>,
}

pub(crate) fn validate_lanes(path: &Path, strict: bool, json: bool) -> Result<()> {
    let path = path.canonicalize().unwrap_or(path.to_path_buf());

    let fledge_toml = path.join("fledge.toml");
    if !fledge_toml.exists() {
        bail!(
            "No fledge.toml found in {}. Point to a directory containing fledge.toml.",
            path.display()
        );
    }

    let content = std::fs::read_to_string(&fledge_toml).context("reading fledge.toml")?;
    let mut report = LaneValidationReport {
        path: path.display().to_string(),
        ..Default::default()
    };

    let parsed: FledgeFileWithLanes = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            report.errors.push(format!("Invalid fledge.toml: {e}"));
            return print_lane_report(&report, strict, json);
        }
    };

    if parsed.lanes.is_empty() {
        report
            .errors
            .push("No [lanes] defined in fledge.toml".to_string());
        return print_lane_report(&report, strict, json);
    }

    report.lane_count = parsed.lanes.len();

    for (name, lane) in &parsed.lanes {
        if lane.steps.is_empty() {
            report.errors.push(format!("Lane '{name}' has no steps"));
        }

        if lane.description.is_none() {
            report
                .warnings
                .push(format!("Lane '{name}' has no description"));
        }

        for (i, step) in lane.steps.iter().enumerate() {
            match step {
                Step::TaskRef(task_name) => {
                    if !parsed.tasks.contains_key(task_name) {
                        report.errors.push(format!(
                            "Lane '{name}' step {} references undefined task '{task_name}'",
                            i + 1
                        ));
                    }
                }
                Step::TaskRefFull {
                    task: task_name, ..
                } => {
                    if !parsed.tasks.contains_key(task_name) {
                        report.errors.push(format!(
                            "Lane '{name}' step {} references undefined task '{task_name}'",
                            i + 1
                        ));
                    }
                }
                Step::Inline { run: cmd, .. } => {
                    if cmd.trim().is_empty() {
                        report.errors.push(format!(
                            "Lane '{name}' step {} has empty inline command",
                            i + 1
                        ));
                    }
                }
                Step::Parallel { parallel, .. } => {
                    if parallel.is_empty() {
                        report.errors.push(format!(
                            "Lane '{name}' step {} has empty parallel group",
                            i + 1
                        ));
                    }
                    for item in parallel {
                        match item {
                            ParallelItem::TaskRef(task_name) => {
                                if !parsed.tasks.contains_key(task_name) {
                                    report.errors.push(format!(
                                        "Lane '{name}' step {} parallel group references undefined task '{task_name}'",
                                        i + 1
                                    ));
                                }
                            }
                            ParallelItem::Inline { run: cmd } => {
                                if cmd.trim().is_empty() {
                                    report.errors.push(format!(
                                        "Lane '{name}' step {} parallel group has empty inline command",
                                        i + 1
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for circular task deps
    for task_name in parsed.tasks.keys() {
        let mut visited = HashSet::new();
        let mut stack = vec![task_name.as_str()];
        while let Some(current) = stack.pop() {
            if !visited.insert(current.to_string()) {
                report.errors.push(format!(
                    "Circular dependency detected involving task '{task_name}'"
                ));
                break;
            }
            if let Some(dep_task) = parsed.tasks.get(current) {
                for dep in dep_task.deps() {
                    stack.push(dep);
                }
            }
        }
    }

    // Check imported lanes in .fledge/lanes/
    let lanes_dir = path.join(".fledge").join("lanes");
    if lanes_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&lanes_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "toml") {
                    let fname = p.file_name().unwrap_or_default().to_string_lossy();
                    match std::fs::read_to_string(&p) {
                        Ok(c) => {
                            if let Err(e) = toml::from_str::<FledgeFileWithLanes>(&c) {
                                report
                                    .errors
                                    .push(format!(".fledge/lanes/{fname}: Invalid TOML: {e}"));
                            }
                        }
                        Err(e) => {
                            report
                                .warnings
                                .push(format!(".fledge/lanes/{fname}: Cannot read: {e}"));
                        }
                    }
                }
            }
        }
    }

    print_lane_report(&report, strict, json)
}

pub(crate) fn print_lane_report(
    report: &LaneValidationReport,
    strict: bool,
    json: bool,
) -> Result<()> {
    if json {
        // Wrap with schema_version envelope (matches lanes list/run/search shape).
        let value = crate::envelope::versioned(1, serde_json::to_value(report)?);
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if report.errors.is_empty() && report.warnings.is_empty() {
        println!(
            "{} {} — valid ({} lanes)",
            style("✅").green().bold(),
            style(&report.path).green(),
            report.lane_count
        );
    } else {
        println!("{}", style(&report.path).bold());
        for e in &report.errors {
            println!("  {} {}", style("error:").red().bold(), e);
        }
        for w in &report.warnings {
            println!("  {} {}", style("warn:").yellow().bold(), w);
        }
    }

    let has_errors = !report.errors.is_empty();
    let has_warnings = !report.warnings.is_empty();
    if has_errors || (strict && has_warnings) {
        bail!("Validation failed");
    }

    Ok(())
}
