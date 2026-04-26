use anyhow::{bail, Context, Result};
use console::style;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::run::detect_project_type;
use crate::trust::{determine_trust_tier, determine_trust_tier_from_owner};

#[derive(Debug, Deserialize)]
struct FledgeFileWithLanes {
    #[serde(default)]
    tasks: BTreeMap<String, TaskDef>,
    #[serde(default)]
    lanes: BTreeMap<String, LaneDef>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TaskDef {
    Short(String),
    Full(TaskConfig),
}

#[derive(Debug, Deserialize)]
struct TaskConfig {
    cmd: String,
    #[serde(default)]
    deps: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    dir: Option<String>,
}

impl TaskDef {
    fn cmd(&self) -> &str {
        match self {
            TaskDef::Short(s) => s,
            TaskDef::Full(c) => &c.cmd,
        }
    }

    fn deps(&self) -> &[String] {
        match self {
            TaskDef::Short(_) => &[],
            TaskDef::Full(c) => &c.deps,
        }
    }

    fn env(&self) -> &BTreeMap<String, String> {
        static EMPTY: BTreeMap<String, String> = BTreeMap::new();
        match self {
            TaskDef::Short(_) => &EMPTY,
            TaskDef::Full(c) => &c.env,
        }
    }

    fn dir(&self) -> Option<&str> {
        match self {
            TaskDef::Short(_) => None,
            TaskDef::Full(c) => c.dir.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LaneDef {
    #[serde(default)]
    description: Option<String>,
    steps: Vec<Step>,
    #[serde(default = "default_true")]
    fail_fast: bool,
    #[serde(skip, default)]
    source: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Step {
    TaskRef(String),
    Inline { run: String },
    Parallel { parallel: Vec<ParallelItem> },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ParallelItem {
    TaskRef(String),
    Inline { run: String },
}

pub enum LaneAction {
    Run {
        name: String,
        dry_run: bool,
        json: bool,
    },
    List {
        json: bool,
    },
    Init {
        json: bool,
    },
    Search {
        query: Option<String>,
        author: Option<String>,
        json: bool,
    },
    Import {
        source: String,
        yes: bool,
        json: bool,
    },
    Publish {
        path: PathBuf,
        org: Option<String>,
        private: bool,
        description: Option<String>,
        yes: bool,
        json: bool,
    },
    Create {
        name: String,
        output: PathBuf,
        description: Option<String>,
        yes: bool,
        json: bool,
    },
    Validate {
        path: PathBuf,
        strict: bool,
        json: bool,
    },
}

pub fn run(action: LaneAction) -> Result<()> {
    match action {
        LaneAction::Search {
            query,
            author,
            json,
        } => search_lanes(query.as_deref(), author.as_deref(), json),
        LaneAction::Import { source, yes, json } => import_lanes(&source, yes, json),
        LaneAction::Init { json } => init_lanes(json),
        LaneAction::Publish {
            path,
            org,
            private,
            description,
            yes,
            json,
        } => publish_lanes(
            &path,
            org.as_deref(),
            private,
            description.as_deref(),
            yes,
            json,
        ),
        LaneAction::Create {
            name,
            output,
            description,
            yes,
            json,
        } => create_lane_repo(&name, &output, description.as_deref(), yes, json),
        LaneAction::Validate { path, strict, json } => validate_lanes(&path, strict, json),
        LaneAction::List { json } => {
            let config = load_lane_config()?;
            list_lanes(&config.lanes, json)
        }
        LaneAction::Run {
            name,
            dry_run,
            json,
        } => {
            let config = load_lane_config()?;
            let lane = config.lanes.get(&name).ok_or_else(|| {
                let available: Vec<&str> = config.lanes.keys().map(|s| s.as_str()).collect();
                anyhow::anyhow!(
                    "Unknown lane '{}'. Available lanes: {}",
                    name,
                    available.join(", ")
                )
            })?;

            if lane.steps.is_empty() {
                bail!("Lane '{}' has no steps defined", name);
            }

            validate_lane(&name, lane, &config.tasks)?;

            if dry_run {
                dry_run_lane(&name, lane)
            } else {
                let project_dir = std::env::current_dir().context("getting current directory")?;
                execute_lane(&name, lane, &config.tasks, &project_dir, json)
            }
        }
    }
}

fn load_lane_config() -> Result<FledgeFileWithLanes> {
    let project_dir = std::env::current_dir().context("getting current directory")?;
    let config_path = project_dir.join("fledge.toml");

    if !config_path.exists() {
        bail!(
            "No fledge.toml found in current directory.\n  Run {} to create one.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&config_path).context("reading fledge.toml")?;
    let mut config: FledgeFileWithLanes = toml::from_str(&content).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("lanes") || msg.contains("steps") {
            anyhow::anyhow!(
                "Error parsing lanes in fledge.toml: {e}\n\n  \
                 Lanes must use table syntax:\n    \
                 [lanes.ci]\n    \
                 steps = [\"lint\", \"test\", \"build\"]\n\n  \
                 Not shorthand like: ci = [\"lint\", \"test\"]"
            )
        } else {
            anyhow::anyhow!("parsing fledge.toml: {e}")
        }
    })?;

    let lanes_dir = project_dir.join(".fledge").join("lanes");
    if lanes_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&lanes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    let imported_content = std::fs::read_to_string(&path)
                        .with_context(|| format!("reading {}", path.display()))?;
                    let imported: FledgeFileWithLanes = toml::from_str(&imported_content)
                        .with_context(|| format!("parsing {}", path.display()))?;
                    let import_source = imported_content
                        .lines()
                        .find(|l| l.starts_with("# Imported from "))
                        .map(|l| l.trim_start_matches("# Imported from ").trim().to_string());
                    for (name, task) in imported.tasks {
                        config.tasks.entry(name).or_insert(task);
                    }
                    for (name, mut lane) in imported.lanes {
                        lane.source = import_source.clone();
                        config.lanes.entry(name).or_insert(lane);
                    }
                }
            }
        }
    }

    if config.lanes.is_empty() {
        bail!(
            "No lanes defined.\n  Add lanes to fledge.toml, import with {}, or run {} to add defaults.",
            style("fledge lanes import <source>").cyan(),
            style("fledge lanes init").cyan()
        );
    }

    Ok(config)
}

fn list_lanes(lanes: &BTreeMap<String, LaneDef>, json: bool) -> Result<()> {
    if json {
        let entries: Vec<serde_json::Value> = lanes
            .iter()
            .map(|(name, lane)| {
                let mut entry = serde_json::json!({
                    "name": name,
                    "description": lane.description,
                    "steps": lane.steps.len(),
                    "fail_fast": lane.fail_fast,
                });
                if let Some(ref src) = lane.source {
                    let tier = determine_trust_tier(src);
                    entry["source"] = serde_json::json!(src);
                    entry["trust_tier"] = serde_json::json!(tier.label());
                } else {
                    entry["trust_tier"] = serde_json::json!("local");
                }
                entry
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    println!("{}", style("Available lanes:").bold());
    let max_name_len = lanes.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, lane) in lanes {
        let desc = lane.description.as_deref().unwrap_or("(no description)");
        let tier_label = match &lane.source {
            Some(src) => {
                let tier = determine_trust_tier(src);
                format!(" [{}]", tier.styled_label())
            }
            None => String::new(),
        };
        println!(
            "  {:<width$}  {}{}",
            style(name).green(),
            style(desc).dim(),
            tier_label,
            width = max_name_len
        );
    }
    Ok(())
}

fn validate_lane(lane_name: &str, lane: &LaneDef, tasks: &BTreeMap<String, TaskDef>) -> Result<()> {
    for (i, step) in lane.steps.iter().enumerate() {
        match step {
            Step::TaskRef(name) => {
                if !tasks.contains_key(name) {
                    bail!(
                        "Lane '{}' step {} references unknown task '{}'.\n  Define it in [tasks] first.",
                        lane_name,
                        i + 1,
                        name
                    );
                }
                check_dep_cycle(name, tasks, &mut HashSet::new())
                    .map_err(|e| anyhow::anyhow!("Lane '{}' step {}: {}", lane_name, i + 1, e))?;
            }
            Step::Inline { .. } => {}
            Step::Parallel { parallel } => {
                for item in parallel {
                    if let ParallelItem::TaskRef(name) = item {
                        if !tasks.contains_key(name) {
                            bail!(
                                "Lane '{}' step {} parallel group references unknown task '{}'.\n  Define it in [tasks] first.",
                                lane_name,
                                i + 1,
                                name
                            );
                        }
                        check_dep_cycle(name, tasks, &mut HashSet::new()).map_err(|e| {
                            anyhow::anyhow!("Lane '{}' step {}: {}", lane_name, i + 1, e)
                        })?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_dep_cycle(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    visiting: &mut HashSet<String>,
) -> Result<()> {
    if !visiting.insert(name.to_string()) {
        bail!("circular dependency detected involving task '{}'", name);
    }
    if let Some(task) = tasks.get(name) {
        for dep in task.deps() {
            check_dep_cycle(dep, tasks, visiting)?;
        }
    }
    visiting.remove(name);
    Ok(())
}

fn dry_run_lane(lane_name: &str, lane: &LaneDef) -> Result<()> {
    let desc = lane.description.as_deref().unwrap_or("(no description)");
    println!(
        "{} {} — {}",
        style("Lane:").bold(),
        style(lane_name).green(),
        style(desc).dim()
    );
    if !lane.fail_fast {
        println!("  {} fail_fast = false", style("⚙").dim());
    }
    for (i, step) in lane.steps.iter().enumerate() {
        match step {
            Step::TaskRef(name) => {
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(name).cyan(),
                    style("(task)").dim()
                );
            }
            Step::Inline { run: cmd } => {
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(cmd).cyan(),
                    style("(inline)").dim()
                );
            }
            Step::Parallel { parallel } => {
                let names: Vec<String> = parallel
                    .iter()
                    .map(|item| match item {
                        ParallelItem::TaskRef(name) => name.clone(),
                        ParallelItem::Inline { run: cmd } => format!("run: {cmd}"),
                    })
                    .collect();
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(names.join(", ")).cyan(),
                    style("(parallel)").dim()
                );
            }
        }
    }
    Ok(())
}

fn execute_lane(
    lane_name: &str,
    lane: &LaneDef,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    json: bool,
) -> Result<()> {
    if json {
        return execute_lane_json(lane_name, lane, tasks, project_dir);
    }

    let desc = lane.description.as_deref().unwrap_or("(no description)");
    println!(
        "{} {} — {}",
        style("▶️ Lane:").cyan().bold(),
        style(lane_name).bold(),
        style(desc).dim()
    );

    let total_steps = lane.steps.len();
    let mut failures: Vec<String> = Vec::new();
    let lane_start = Instant::now();

    for (i, step) in lane.steps.iter().enumerate() {
        let step_start = Instant::now();
        let result = match step {
            Step::TaskRef(name) => execute_task_with_deps(name, tasks, project_dir),
            Step::Inline { run: cmd } => execute_inline(cmd, project_dir),
            Step::Parallel { parallel } => execute_parallel(parallel, tasks, project_dir),
        };
        let elapsed = step_start.elapsed();

        if let Err(e) = result {
            let step_desc = match step {
                Step::TaskRef(name) => name.clone(),
                Step::Inline { run: cmd } => cmd.clone(),
                Step::Parallel { parallel } => {
                    let names: Vec<String> = parallel
                        .iter()
                        .map(|item| match item {
                            ParallelItem::TaskRef(name) => name.clone(),
                            ParallelItem::Inline { run: cmd } => cmd.clone(),
                        })
                        .collect();
                    format!("parallel({})", names.join(", "))
                }
            };
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
) -> Result<()> {
    let total_steps = lane.steps.len();
    let mut step_results: Vec<serde_json::Value> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let lane_start = Instant::now();

    for (i, step) in lane.steps.iter().enumerate() {
        let step_desc = step_description(step);
        let step_start = Instant::now();
        let result = match step {
            Step::TaskRef(name) => execute_task_with_deps(name, tasks, project_dir),
            Step::Inline { run: cmd } => execute_inline(cmd, project_dir),
            Step::Parallel { parallel } => execute_parallel(parallel, tasks, project_dir),
        };
        let elapsed = step_start.elapsed();
        let success = result.is_ok();
        let error_msg = result.err().map(|e| e.to_string());

        step_results.push(serde_json::json!({
            "step": i + 1,
            "name": step_desc,
            "success": success,
            "duration_ms": elapsed.as_millis() as u64,
            "error": error_msg,
        }));

        if !success {
            failures.push(step_desc.clone());
            if lane.fail_fast {
                break;
            }
        }
    }

    let total_elapsed = lane_start.elapsed();
    let success = failures.is_empty();

    let output = serde_json::json!({
        "lane": lane_name,
        "description": lane.description.as_deref().unwrap_or(""),
        "total_steps": total_steps,
        "success": success,
        "duration_ms": total_elapsed.as_millis() as u64,
        "fail_fast": lane.fail_fast,
        "steps": step_results,
        "failures": failures,
    });

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

fn step_description(step: &Step) -> String {
    match step {
        Step::TaskRef(name) => name.clone(),
        Step::Inline { run: cmd } => cmd.clone(),
        Step::Parallel { parallel } => {
            let names: Vec<String> = parallel
                .iter()
                .map(|item| match item {
                    ParallelItem::TaskRef(name) => name.clone(),
                    ParallelItem::Inline { run: cmd } => cmd.clone(),
                })
                .collect();
            format!("parallel({})", names.join(", "))
        }
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    if secs >= 60 {
        let mins = secs / 60;
        let remaining = secs % 60;
        format!("{mins}m {remaining}.{millis:03}s")
    } else if secs > 0 {
        format!("{secs}.{millis:03}s")
    } else {
        format!("{millis}ms")
    }
}

fn execute_task_with_deps(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
) -> Result<()> {
    let mut visited = HashSet::new();
    execute_task_recursive(name, tasks, project_dir, &mut visited)
}

fn execute_task_recursive(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    visited: &mut HashSet<String>,
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
        execute_task_recursive(dep, tasks, project_dir, visited)?;
    }

    execute_single_task(name, task, project_dir)
}

fn execute_single_task(name: &str, task: &TaskDef, project_dir: &Path) -> Result<()> {
    println!(
        "  {} {}",
        style("▶️").cyan().bold(),
        style(format!("Running task: {name}")).bold()
    );

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

    let status = command
        .status()
        .with_context(|| format!("running task '{name}'"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Task '{}' failed with exit code {}", name, code);
    }

    Ok(())
}

fn execute_inline(cmd: &str, project_dir: &Path) -> Result<()> {
    println!(
        "  {} {}",
        style("▶️").cyan().bold(),
        style(format!("Running: {cmd}")).bold()
    );

    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    let status = Command::new(shell)
        .arg(flag)
        .arg(cmd)
        .current_dir(project_dir)
        .status()
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
) -> Result<()> {
    let names_display: Vec<String> = items
        .iter()
        .map(|item| match item {
            ParallelItem::TaskRef(name) => name.clone(),
            ParallelItem::Inline { run: cmd } => cmd.clone(),
        })
        .collect();
    println!(
        "  {} {}",
        style("▶️").cyan().bold(),
        style(format!("Running parallel: {}", names_display.join(", "))).bold()
    );

    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|s| {
        let mut handles = Vec::new();

        for item in items {
            let errors = Arc::clone(&errors);
            let handle = s.spawn(move || {
                let result = match item {
                    ParallelItem::TaskRef(name) => execute_task_with_deps(name, tasks, project_dir),
                    ParallelItem::Inline { run: cmd } => execute_inline(cmd, project_dir),
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

fn lane_defaults(project_type: &str) -> &'static str {
    match project_type {
        "rust" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "node" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["lint", "test"] },
]
"#
        }
        "go" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "python" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "swift" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["build", "test"]
"#
        }
        _ => {
            r#"
# [lanes.ci]
# description = "Run full CI pipeline"
# steps = ["lint", "test", "build"]
"#
        }
    }
}

fn init_lanes(json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = cwd.join("fledge.toml");

    if !path.exists() {
        bail!(
            "No fledge.toml found. Run {} first, then add lanes.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&path).context("reading fledge.toml")?;

    if content.contains("[lanes") {
        bail!("Lanes already defined in fledge.toml. Edit them manually.");
    }

    let project_type = detect_project_type(&cwd);
    let defaults = lane_defaults(project_type);

    let new_content = format!("{}{}", content.trim_end(), defaults);
    std::fs::write(&path, new_content).context("writing fledge.toml")?;

    if json {
        let lanes_added: Vec<&str> = defaults
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("[lanes.")
                    .and_then(|s| s.strip_suffix(']'))
            })
            .collect();
        let output = serde_json::json!({
            "schema_version": 1,
            "action": "init",
            "project_type": project_type,
            "lanes_added": lanes_added,
            "file": "fledge.toml"
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "{} Added default lanes to {}",
            style("✅").green().bold(),
            style("fledge.toml").cyan()
        );
        println!("  Run {} to see them.", style("fledge lane").cyan());
    }
    Ok(())
}

fn search_lanes(keyword: Option<&str>, author: Option<&str>, json: bool) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let query = crate::search::build_search_query_ex(keyword, author, "fledge-lane");

    let sp = crate::spinner::Spinner::start("Searching GitHub for community lanes:");

    let body = crate::github::github_api_get(
        "/search/repositories",
        token.as_deref(),
        &[("q", &query), ("sort", "stars"), ("per_page", "30")],
    )
    .context("searching GitHub for lane repos")?;

    sp.finish();

    let results = crate::search::parse_search_response(&body)?;

    if results.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "{} No community lanes found{}.",
                style("*").cyan().bold(),
                keyword
                    .map(|q| format!(" matching '{q}'"))
                    .unwrap_or_default()
            );
        }
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let tier = determine_trust_tier_from_owner(&r.owner);
                serde_json::json!({
                    "owner": r.owner,
                    "name": r.name,
                    "description": r.description,
                    "stars": r.stars,
                    "url": r.url,
                    "topics": r.topics,
                    "trust_tier": tier.label(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    println!("{}\n", style("Community lanes on GitHub:").bold());
    let max_name = results
        .iter()
        .map(|r| r.full_name().len())
        .max()
        .unwrap_or(0);
    for r in &results {
        let tier = determine_trust_tier_from_owner(&r.owner);
        let stars = crate::search::format_stars(r.stars);
        let desc = if r.description.chars().count() > 60 {
            let truncated: String = r.description.chars().take(57).collect();
            format!("{truncated}...")
        } else {
            r.description.clone()
        };
        let topic_str = if r.topics.is_empty() {
            String::new()
        } else {
            format!(" [{}]", r.topics.join(", "))
        };
        println!(
            "  {:<width$}  [{}]  {}  {}{}",
            style(&r.full_name()).green(),
            tier.styled_label(),
            style(format!("(⭐ {})", stars)).dim(),
            style(&desc).dim(),
            style(&topic_str).cyan(),
            width = max_name,
        );
    }
    println!(
        "\n{}",
        style("Import with: fledge lane import <owner/repo[/path]>").dim()
    );

    Ok(())
}

fn import_lanes(source: &str, _yes: bool, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let local_path = cwd.join("fledge.toml");

    if !local_path.exists() {
        bail!(
            "No fledge.toml found. Run {} first.",
            style("fledge run --init").cyan()
        );
    }

    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let (owner, repo, subpath, git_ref) = parse_import_source(source);

    let display_source = format!(
        "{}/{}{}{}",
        owner,
        repo,
        subpath
            .as_ref()
            .map(|p| format!("/{p}"))
            .unwrap_or_default(),
        git_ref
            .as_ref()
            .map(|r| format!("@{r}"))
            .unwrap_or_default()
    );

    let tier = determine_trust_tier(&display_source);
    if !json {
        println!(
            "\n{} Importing lanes from: {} [{}]",
            style("!").yellow().bold(),
            style(&display_source).cyan(),
            tier.styled_label()
        );
        if tier != crate::trust::TrustTier::Official {
            println!(
                "  {} Lanes can execute arbitrary commands on your system.",
                style("*").yellow()
            );
            println!(
                "  {} Only import lanes from sources you trust.\n",
                style("*").yellow()
            );
        }
    }

    let sp = if !json {
        Some(crate::spinner::Spinner::start(&format!(
            "Fetching lanes from {}:",
            display_source,
        )))
    } else {
        None
    };

    let ref_param = git_ref.as_deref().unwrap_or("HEAD");
    let remote_path = match &subpath {
        Some(p) => format!("{p}/fledge.toml"),
        None => "fledge.toml".to_string(),
    };
    let body = crate::github::github_api_get(
        &format!("/repos/{owner}/{repo}/contents/{remote_path}"),
        token.as_deref(),
        &[("ref", ref_param)],
    )
    .context(format!("fetching {remote_path} from remote repo"))?;

    if let Some(sp) = sp {
        sp.finish();
    }

    let content_b64 = body
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("Remote repo has no fledge.toml or it's not a file"))?;

    let cleaned: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64_decode(&cleaned).context("decoding fledge.toml content")?;
    let remote_content = String::from_utf8(decoded).context("fledge.toml is not valid UTF-8")?;

    let remote_config: FledgeFileWithLanes =
        toml::from_str(&remote_content).context("parsing remote fledge.toml")?;

    if remote_config.lanes.is_empty() {
        bail!("Remote repo has no [lanes] defined in fledge.toml.");
    }

    let existing = load_lane_config()?;

    let mut imported_lanes = Vec::new();
    let mut skipped = Vec::new();
    let mut import_content = String::new();

    import_content.push_str(&format!("# Imported from {display_source}\n\n"));

    for (task_name, task_def) in &remote_config.tasks {
        if existing.tasks.contains_key(task_name) {
            continue;
        }
        let cmd = escape_toml_value(task_def.cmd());
        import_content.push_str(&format!("[tasks.{task_name}]\ncmd = \"{cmd}\"\n\n"));
    }

    for (lane_name, lane) in &remote_config.lanes {
        if existing.lanes.contains_key(lane_name) {
            skipped.push(lane_name.clone());
            continue;
        }
        import_content.push_str(&format_lane_toml(lane_name, lane));
        imported_lanes.push(lane_name.clone());
    }

    let import_warnings: Vec<&str> = if tier != crate::trust::TrustTier::Official {
        vec!["unverified source — lanes can execute arbitrary commands on your system"]
    } else {
        vec![]
    };

    if imported_lanes.is_empty() {
        if json {
            let output = serde_json::json!({
                "schema_version": 1,
                "action": "import",
                "source": display_source,
                "imported": [],
                "skipped": skipped,
                "file": null,
                "warnings": import_warnings,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!(
                "{} All lanes from {} already exist locally ({})",
                style("*").cyan().bold(),
                display_source,
                skipped.join(", ")
            );
        }
        return Ok(());
    }

    let lanes_dir = cwd.join(".fledge").join("lanes");
    std::fs::create_dir_all(&lanes_dir).context("creating .fledge/lanes directory")?;

    let safe_name = format!(
        "{}-{}{}",
        owner.to_lowercase(),
        repo.to_lowercase(),
        subpath
            .as_ref()
            .map(|p| format!("-{}", p.replace('/', "-").to_lowercase()))
            .unwrap_or_default()
    );
    let import_path = lanes_dir.join(format!("{safe_name}.toml"));
    std::fs::write(&import_path, import_content.trim_start()).context("writing imported lanes")?;

    let import_file = format!(".fledge/lanes/{safe_name}.toml");

    if json {
        let output = serde_json::json!({
            "schema_version": 1,
            "action": "import",
            "source": display_source,
            "imported": imported_lanes,
            "skipped": skipped,
            "file": import_file,
            "warnings": import_warnings,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "{} Imported {} lane(s) from {}",
            style("✅").green().bold(),
            imported_lanes.len(),
            display_source
        );
        for name in &imported_lanes {
            println!("  {} {}", style("+").green(), style(name).cyan());
        }
        println!(
            "  {} Saved to {}",
            style("→").dim(),
            style(&import_file).cyan()
        );
        if !skipped.is_empty() {
            println!(
                "  {} Skipped (already exist): {}",
                style("*").dim(),
                skipped.join(", ")
            );
        }
    }

    Ok(())
}

fn parse_import_source(source: &str) -> (String, String, Option<String>, Option<String>) {
    let source = source
        .strip_prefix("https://github.com/")
        .unwrap_or(source)
        .trim_end_matches(".git");

    let (path, git_ref) = if let Some((p, r)) = source.split_once('@') {
        (p, Some(r.to_string()))
    } else {
        (source, None)
    };

    let parts: Vec<&str> = path.splitn(3, '/').collect();
    let owner = parts.first().unwrap_or(&"").to_string();
    let repo = parts.get(1).unwrap_or(&"").to_string();
    let subpath = parts
        .get(2)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    (owner, repo, subpath, git_ref)
}

fn escape_toml_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_lane_toml(name: &str, lane: &LaneDef) -> String {
    let mut out = format!("\n[lanes.{}]\n", name);
    if let Some(ref desc) = lane.description {
        out.push_str(&format!("description = \"{}\"\n", escape_toml_value(desc)));
    }
    if !lane.fail_fast {
        out.push_str("fail_fast = false\n");
    }
    out.push_str("steps = [");
    for (i, step) in lane.steps.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match step {
            Step::TaskRef(name) => {
                out.push_str(&format!("\"{}\"", escape_toml_value(name)));
            }
            Step::Inline { run: cmd } => {
                out.push_str(&format!("{{ run = \"{}\" }}", escape_toml_value(cmd)));
            }
            Step::Parallel { parallel } => {
                let items: Vec<String> = parallel
                    .iter()
                    .map(|item| match item {
                        ParallelItem::TaskRef(name) => {
                            format!("\"{}\"", escape_toml_value(name))
                        }
                        ParallelItem::Inline { run: cmd } => {
                            format!("{{ run = \"{}\" }}", escape_toml_value(cmd))
                        }
                    })
                    .collect();
                out.push_str(&format!("{{ parallel = [{}] }}", items.join(", ")));
            }
        }
    }
    out.push_str("]\n");
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u32 - b'A' as u32,
            'a'..='z' => c as u32 - b'a' as u32 + 26,
            '0'..='9' => c as u32 - b'0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            '=' => break,
            _ => continue,
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(output)
}

fn create_lane_repo(
    name: &str,
    output: &Path,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let target = output.join(name);

    if target.exists() {
        bail!("Directory '{}' already exists", target.display());
    }

    let desc = if yes || json || !crate::utils::is_interactive() {
        description.unwrap_or("Shared fledge lanes").to_string()
    } else {
        let theme = dialoguer::theme::ColorfulTheme::default();
        dialoguer::Input::with_theme(&theme)
            .with_prompt("Description")
            .default(description.unwrap_or("Shared fledge lanes").to_string())
            .interact_text()?
    };

    std::fs::create_dir_all(&target).with_context(|| format!("creating {}", target.display()))?;

    let fledge_toml = format!(
        r#"[tasks]
lint = "echo 'lint placeholder'"
test = "echo 'test placeholder'"
build = "echo 'build placeholder'"
fmt = "echo 'fmt placeholder'"

[lanes.ci]
description = {desc:?}
steps = ["lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  {{ parallel = ["lint", "fmt"] }},
  "test"
]
"#,
        desc = format!("{name} CI pipeline")
    );
    std::fs::write(target.join("fledge.toml"), fledge_toml).context("writing fledge.toml")?;

    std::fs::write(
        target.join("README.md"),
        format!(
            r#"# {name} — fledge lanes

{desc}

## Usage

Import these lanes into any fledge project:

```bash
fledge lanes import ./{name}
```

Or after publishing:

```bash
fledge lanes import owner/{name}
```

## Lanes

| Lane | Description |
|------|-------------|
| `ci` | {name} CI pipeline |
| `check` | Quick quality check |

## Customization

Edit `fledge.toml` to add, modify, or remove lanes and tasks.
See [fledge docs](https://github.com/CorvidLabs/fledge) for lane syntax.
"#
        ),
    )
    .context("writing README.md")?;

    std::fs::write(target.join(".gitignore"), "# OS\n.DS_Store\nThumbs.db\n")
        .context("writing .gitignore")?;

    if json {
        let output = serde_json::json!({
            "schema_version": 1,
            "action": "create",
            "path": target.display().to_string(),
            "name": name,
            "description": desc,
            "files_created": ["fledge.toml", "README.md", ".gitignore"]
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "\n{} Created lane repo at {}",
            style("✅").green().bold(),
            style(target.display()).cyan()
        );
        println!(
            "\n  {} Edit lanes in {}",
            style("1.").dim(),
            style("fledge.toml").green()
        );
        println!(
            "  {} Validate with: {}",
            style("2.").dim(),
            style(format!("fledge lanes validate ./{name}")).cyan()
        );
        println!(
            "  {} Publish with: {}",
            style("3.").dim(),
            style(format!("fledge lanes publish ./{name}")).cyan()
        );
    }

    Ok(())
}

#[derive(Default, serde::Serialize)]
struct LaneValidationReport {
    path: String,
    lane_count: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_lanes(path: &Path, strict: bool, json: bool) -> Result<()> {
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
                Step::Inline { run: cmd } => {
                    if cmd.trim().is_empty() {
                        report.errors.push(format!(
                            "Lane '{name}' step {} has empty inline command",
                            i + 1
                        ));
                    }
                }
                Step::Parallel { parallel } => {
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

fn print_lane_report(report: &LaneValidationReport, strict: bool, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
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

fn publish_lanes(
    path: &Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let yes = yes || json || crate::utils::is_non_interactive();
    let config = crate::config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;

    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;

    let fledge_toml = path.join("fledge.toml");
    if !fledge_toml.exists() {
        bail!(
            "No fledge.toml found in {}. Lanes must be defined in a fledge.toml file.",
            path.display()
        );
    }

    validate_lanes(&path, false, json)?;

    let content = std::fs::read_to_string(&fledge_toml).context("reading fledge.toml")?;
    let parsed: FledgeFileWithLanes = toml::from_str(&content).context("parsing fledge.toml")?;

    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fledge-lanes");
    let repo_name = dir_name.to_string();
    let desc = description.unwrap_or("Shared fledge lanes");

    let owner = match org {
        Some(o) => o.to_string(),
        None => crate::publish::get_authenticated_user(&token)?,
    };

    let lane_names: Vec<&str> = parsed.lanes.keys().map(|s| s.as_str()).collect();
    if !json {
        println!(
            "{} Publishing {} lanes as {}/{}",
            style("➡️").cyan().bold(),
            style(lane_names.len()).green(),
            style(&owner).green(),
            style(&repo_name).green()
        );
        println!("  Lanes: {}", style(lane_names.join(", ")).dim());
    }

    let sp = if !json {
        Some(crate::spinner::Spinner::start("Checking repository:"))
    } else {
        None
    };
    let repo_exists = crate::publish::check_repo_exists(&owner, &repo_name, &token)?;
    if let Some(sp) = sp {
        sp.finish();
    }

    if repo_exists {
        if !yes {
            crate::utils::require_interactive("yes")?;
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(format!(
                        "Repository {}/{} already exists. Push update?",
                        owner, repo_name
                    ))
                    .default(false)
                    .interact()?;

            if !confirm {
                if !json {
                    println!("{} Cancelled.", style("*").cyan().bold());
                }
                return Ok(());
            }
        }
    } else {
        let sp = if !json {
            Some(crate::spinner::Spinner::start("Creating repository:"))
        } else {
            None
        };
        crate::publish::create_github_repo(&repo_name, desc, private, org, &token)?;
        if let Some(sp) = sp {
            sp.finish();
        }
        if !json {
            println!(
                "  {} Created repository {}/{}",
                style("✅").green().bold(),
                owner,
                repo_name
            );
        }
    }

    let sp = if !json {
        Some(crate::spinner::Spinner::start("Setting repository topics:"))
    } else {
        None
    };
    crate::publish::set_repo_topic(&owner, &repo_name, "fledge-lane", &token)?;
    if let Some(sp) = sp {
        sp.finish();
    }
    if !json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style("fledge-lane").cyan()
        );
    }

    let sp = if !json {
        Some(crate::spinner::Spinner::start("Pushing lane files:"))
    } else {
        None
    };
    crate::publish::push_directory(&path, &owner, &repo_name, &token)?;
    if let Some(sp) = sp {
        sp.finish();
    }
    if !json {
        println!("  {} Pushed lane files", style("✅").green().bold());
    }

    let visibility = if private { "private" } else { "public" };
    let repo_url = format!("https://github.com/{}/{}", owner, repo_name);

    if json {
        let output = serde_json::json!({
            "schema_version": 1,
            "action": "publish",
            "repo": {
                "owner": owner,
                "name": repo_name,
                "url": repo_url
            },
            "visibility": visibility,
            "lanes_published": lane_names.len()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "\n{} Published! Import with:\n\n  {}",
            style("✅").green().bold(),
            style(format!("fledge lanes import {}/{}", owner, repo_name)).cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(toml_str: &str) -> FledgeFileWithLanes {
        toml::from_str(toml_str).unwrap()
    }

    #[test]
    fn parse_sequential_lane() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[lanes.ci]
description = "CI pipeline"
steps = ["lint", "test", "build"]
"#,
        );
        assert_eq!(config.lanes.len(), 1);
        assert_eq!(config.lanes["ci"].steps.len(), 3);
        assert!(config.lanes["ci"].fail_fast);
    }

    #[test]
    fn parse_inline_step() {
        let config = parse_config(
            r#"
[tasks]
test = "cargo test"

[lanes.release]
description = "Release"
steps = [
  "test",
  { run = "cargo build --release" },
]
"#,
        );
        assert_eq!(config.lanes["release"].steps.len(), 2);
        match &config.lanes["release"].steps[1] {
            Step::Inline { run: cmd } => assert_eq!(cmd, "cargo build --release"),
            _ => panic!("expected inline step"),
        }
    }

    #[test]
    fn parse_parallel_step() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"
fmt = "cargo fmt --check"
test = "cargo test"

[lanes.check]
description = "Quick check"
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]
"#,
        );
        assert_eq!(config.lanes["check"].steps.len(), 2);
        match &config.lanes["check"].steps[0] {
            Step::Parallel { parallel } => {
                assert_eq!(parallel.len(), 2);
                assert!(matches!(&parallel[0], ParallelItem::TaskRef(n) if n == "lint"));
                assert!(matches!(&parallel[1], ParallelItem::TaskRef(n) if n == "fmt"));
            }
            _ => panic!("expected parallel step"),
        }
    }

    #[test]
    fn parse_fail_fast_false() {
        let config = parse_config(
            r#"
[tasks]
a = "echo a"
b = "echo b"

[lanes.audit]
description = "Audit"
fail_fast = false
steps = ["a", "b"]
"#,
        );
        assert!(!config.lanes["audit"].fail_fast);
    }

    #[test]
    fn parse_fail_fast_default_true() {
        let config = parse_config(
            r#"
[tasks]
a = "echo a"

[lanes.ci]
steps = ["a"]
"#,
        );
        assert!(config.lanes["ci"].fail_fast);
    }

    #[test]
    fn validate_unknown_task_ref() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
steps = ["lint", "nonexistent"]
"#,
        );
        let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn validate_unknown_parallel_ref() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"

[lanes.check]
steps = [{ parallel = ["lint", "ghost"] }]
"#,
        );
        let result = validate_lane("check", &config.lanes["check"], &config.tasks);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ghost"));
    }

    #[test]
    fn validate_inline_always_ok() {
        let config = parse_config(
            r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo hello" }]
"#,
        );
        let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_all_valid_refs() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[lanes.ci]
steps = ["lint", "test", "build"]
"#,
        );
        let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_circular_deps() {
        let config = parse_config(
            r#"
[tasks.a]
cmd = "echo a"
deps = ["b"]

[tasks.b]
cmd = "echo b"
deps = ["a"]

[lanes.ci]
steps = ["a"]
"#,
        );
        let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("circular"),
            "expected circular error, got: {err}"
        );
    }

    #[test]
    fn validate_no_cycle_with_shared_deps() {
        let config = parse_config(
            r#"
[tasks]
common = "echo common"

[tasks.a]
cmd = "echo a"
deps = ["common"]

[tasks.b]
cmd = "echo b"
deps = ["common"]

[lanes.ci]
steps = ["a", "b"]
"#,
        );
        let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_multiple_lanes() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[lanes.ci]
description = "CI"
steps = ["lint", "test", "build"]

[lanes.quick]
description = "Quick"
steps = ["lint"]
"#,
        );
        assert_eq!(config.lanes.len(), 2);
        assert!(config.lanes.contains_key("ci"));
        assert!(config.lanes.contains_key("quick"));
    }

    #[test]
    fn parse_no_lanes_section() {
        let config = parse_config(
            r#"
[tasks]
build = "cargo build"
"#,
        );
        assert!(config.lanes.is_empty());
    }

    #[test]
    fn parse_empty_lanes_section() {
        let config = parse_config(
            r#"
[tasks]
build = "cargo build"

[lanes]
"#,
        );
        assert!(config.lanes.is_empty());
    }

    #[test]
    fn parse_mixed_step_types() {
        let config = parse_config(
            r#"
[tasks]
test = "cargo test"
lint = "cargo clippy"

[lanes.full]
steps = [
  "test",
  { run = "echo done" },
  { parallel = ["test", "lint"] },
]
"#,
        );
        assert_eq!(config.lanes["full"].steps.len(), 3);
        assert!(matches!(&config.lanes["full"].steps[0], Step::TaskRef(_)));
        assert!(matches!(
            &config.lanes["full"].steps[1],
            Step::Inline { .. }
        ));
        assert!(matches!(
            &config.lanes["full"].steps[2],
            Step::Parallel { .. }
        ));
    }

    #[test]
    fn execute_sequential_lane_echo() {
        let config = parse_config(
            r#"
[tasks]
a = "echo step-a"
b = "echo step-b"

[lanes.seq]
description = "Sequential"
steps = ["a", "b"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "seq",
            &config.lanes["seq"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn execute_inline_step() {
        let config = parse_config(
            r#"
[tasks]

[lanes.inline]
steps = [{ run = "echo inline-works" }]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "inline",
            &config.lanes["inline"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn execute_parallel_step() {
        let config = parse_config(
            r#"
[tasks]
a = "echo parallel-a"
b = "echo parallel-b"

[lanes.par]
steps = [{ parallel = ["a", "b"] }]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "par",
            &config.lanes["par"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn parse_parallel_inline_items() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"

[lanes.mixed]
description = "Mixed parallel"
steps = [
  { parallel = ["lint", { run = "echo inline" }] },
]
"#,
        );
        match &config.lanes["mixed"].steps[0] {
            Step::Parallel { parallel } => {
                assert_eq!(parallel.len(), 2);
                assert!(matches!(&parallel[0], ParallelItem::TaskRef(n) if n == "lint"));
                assert!(
                    matches!(&parallel[1], ParallelItem::Inline { run } if run == "echo inline")
                );
            }
            _ => panic!("expected parallel step"),
        }
    }

    #[test]
    fn execute_parallel_with_inline() {
        let config = parse_config(
            r#"
[tasks]
a = "echo task-a"

[lanes.mixed]
steps = [{ parallel = ["a", { run = "echo inline-b" }] }]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "mixed",
            &config.lanes["mixed"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn execute_parallel_all_inline() {
        let config = parse_config(
            r#"
[tasks]

[lanes.inlines]
steps = [{ parallel = [{ run = "echo one" }, { run = "echo two" }] }]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "inlines",
            &config.lanes["inlines"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_parallel_inline_no_task_check() {
        let config = parse_config(
            r#"
[tasks]

[lanes.ci]
steps = [{ parallel = [{ run = "echo hello" }] }]
"#,
        );
        let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn format_lane_toml_with_parallel_inline() {
        let lane = LaneDef {
            description: None,
            steps: vec![Step::Parallel {
                parallel: vec![
                    ParallelItem::TaskRef("lint".to_string()),
                    ParallelItem::Inline {
                        run: "echo done".to_string(),
                    },
                ],
            }],
            fail_fast: true,
            source: None,
        };
        let toml = format_lane_toml("mixed", &lane);
        assert!(toml.contains("\"lint\""));
        assert!(toml.contains("{ run = \"echo done\" }"));
    }

    #[test]
    fn execute_fail_fast_stops() {
        let config = parse_config(
            r#"
[tasks]
fail = "exit 1"
ok = "echo ok"

[lanes.ff]
fail_fast = true
steps = ["fail", "ok"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "ff",
            &config.lanes["ff"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed at step 1"));
    }

    #[test]
    fn execute_no_fail_fast_continues() {
        let config = parse_config(
            r#"
[tasks]
fail = "exit 1"
ok = "echo ok"

[lanes.noff]
fail_fast = false
steps = ["fail", "ok"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "noff",
            &config.lanes["noff"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("1 failure"));
    }

    #[test]
    fn execute_task_deps_in_lane() {
        let config = parse_config(
            r#"
[tasks.build]
cmd = "echo building"
deps = ["prep"]

[tasks.prep]
cmd = "echo preparing"

[lanes.ci]
steps = ["build"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_lane(
            "ci",
            &config.lanes["ci"],
            &config.tasks,
            &project_dir,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn lane_defaults_are_valid_toml() {
        for project_type in &["rust", "node", "go", "python", "generic"] {
            let tasks = match *project_type {
                "rust" => {
                    "[tasks]\nfmt = \"cargo fmt\"\nlint = \"cargo clippy\"\ntest = \"cargo test\"\nbuild = \"cargo build\"\ntypecheck = \"echo ok\"\n"
                }
                "node" => {
                    "[tasks]\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n"
                }
                "go" => {
                    "[tasks]\nfmt = \"echo fmt\"\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n"
                }
                "python" => {
                    "[tasks]\nfmt = \"echo fmt\"\nlint = \"echo lint\"\ntypecheck = \"echo tc\"\ntest = \"echo test\"\n"
                }
                _ => {
                    "[tasks]\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n"
                }
            };
            let defaults = lane_defaults(project_type);
            let toml_str = format!("{}{}", tasks, defaults);
            let result: Result<FledgeFileWithLanes, _> = toml::from_str(&toml_str);
            assert!(
                result.is_ok(),
                "Invalid TOML for {}: {:?}",
                project_type,
                result.err()
            );
        }
    }

    #[test]
    fn parse_import_source_basic() {
        let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-lanes");
        assert!(subpath.is_none());
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_import_source_with_ref() {
        let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes@v1.0.0");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-lanes");
        assert!(subpath.is_none());
        assert_eq!(git_ref.unwrap(), "v1.0.0");
    }

    #[test]
    fn parse_import_source_with_subpath() {
        let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes/rust");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-lanes");
        assert_eq!(subpath.unwrap(), "rust");
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_import_source_with_subpath_and_ref() {
        let (owner, repo, subpath, git_ref) =
            parse_import_source("CorvidLabs/fledge-lanes/rust@main");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-lanes");
        assert_eq!(subpath.unwrap(), "rust");
        assert_eq!(git_ref.unwrap(), "main");
    }

    #[test]
    fn parse_import_source_full_url() {
        let (owner, repo, subpath, git_ref) =
            parse_import_source("https://github.com/CorvidLabs/fledge-lanes.git");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-lanes");
        assert!(subpath.is_none());
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_import_source_url_with_ref() {
        let (owner, repo, subpath, git_ref) =
            parse_import_source("https://github.com/CorvidLabs/fledge-lanes@main");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-lanes");
        assert!(subpath.is_none());
        assert_eq!(git_ref.unwrap(), "main");
    }

    #[test]
    fn format_lane_toml_sequential() {
        let lane = LaneDef {
            description: Some("CI pipeline".to_string()),
            steps: vec![
                Step::TaskRef("lint".to_string()),
                Step::TaskRef("test".to_string()),
            ],
            fail_fast: true,
            source: None,
        };
        let toml = format_lane_toml("ci", &lane);
        assert!(toml.contains("[lanes.ci]"));
        assert!(toml.contains("description = \"CI pipeline\""));
        assert!(toml.contains("\"lint\""));
        assert!(toml.contains("\"test\""));
        assert!(!toml.contains("fail_fast"));
    }

    #[test]
    fn format_lane_toml_with_fail_fast_false() {
        let lane = LaneDef {
            description: None,
            steps: vec![Step::TaskRef("audit".to_string())],
            fail_fast: false,
            source: None,
        };
        let toml = format_lane_toml("audit", &lane);
        assert!(toml.contains("fail_fast = false"));
    }

    #[test]
    fn format_lane_toml_with_inline() {
        let lane = LaneDef {
            description: None,
            steps: vec![Step::Inline {
                run: "echo hello".to_string(),
            }],
            fail_fast: true,
            source: None,
        };
        let toml = format_lane_toml("test", &lane);
        assert!(toml.contains("{ run = \"echo hello\" }"));
    }

    #[test]
    fn format_lane_toml_with_parallel() {
        let lane = LaneDef {
            description: None,
            steps: vec![Step::Parallel {
                parallel: vec![
                    ParallelItem::TaskRef("lint".to_string()),
                    ParallelItem::TaskRef("fmt".to_string()),
                ],
            }],
            fail_fast: true,
            source: None,
        };
        let toml = format_lane_toml("check", &lane);
        assert!(toml.contains("parallel"));
        assert!(toml.contains("\"lint\""));
        assert!(toml.contains("\"fmt\""));
    }

    #[test]
    fn format_lane_toml_roundtrips() {
        let lane = LaneDef {
            description: Some("Full CI".to_string()),
            steps: vec![
                Step::TaskRef("lint".to_string()),
                Step::TaskRef("test".to_string()),
                Step::TaskRef("build".to_string()),
            ],
            fail_fast: true,
            source: None,
        };
        let toml_str = format!(
            "[tasks]\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n{}",
            format_lane_toml("ci", &lane)
        );
        let parsed: FledgeFileWithLanes = toml::from_str(&toml_str).unwrap();
        assert!(parsed.lanes.contains_key("ci"));
        assert_eq!(parsed.lanes["ci"].steps.len(), 3);
    }

    #[test]
    fn base64_decode_basic() {
        let encoded = "SGVsbG8gV29ybGQ=";
        let decoded = base64_decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello World");
    }

    #[test]
    fn base64_decode_no_padding() {
        let encoded = "Zm9v";
        let decoded = base64_decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "foo");
    }

    #[test]
    fn base64_decode_empty() {
        let decoded = base64_decode("").unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn base64_decode_with_newlines() {
        let encoded = "SGVs\nbG8=";
        let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
        let decoded = base64_decode(&cleaned).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello");
    }

    #[test]
    fn format_duration_millis() {
        let d = std::time::Duration::from_millis(42);
        assert_eq!(format_duration(d), "42ms");
    }

    #[test]
    fn format_duration_seconds() {
        let d = std::time::Duration::from_millis(3456);
        assert_eq!(format_duration(d), "3.456s");
    }

    #[test]
    fn format_duration_minutes() {
        let d = std::time::Duration::from_secs(125) + std::time::Duration::from_millis(100);
        assert_eq!(format_duration(d), "2m 5.100s");
    }

    #[test]
    fn format_duration_zero() {
        let d = std::time::Duration::from_millis(0);
        assert_eq!(format_duration(d), "0ms");
    }

    #[test]
    fn merge_imported_lanes() {
        let mut base = parse_config(
            r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
steps = ["lint"]
"#,
        );
        let imported = parse_config(
            r#"
[tasks]
lint = "overridden"
test = "cargo test"

[lanes.ci]
steps = ["lint", "test"]

[lanes.deploy]
steps = ["test"]
"#,
        );

        for (name, task) in imported.tasks {
            base.tasks.entry(name).or_insert(task);
        }
        for (name, lane) in imported.lanes {
            base.lanes.entry(name).or_insert(lane);
        }

        assert_eq!(base.tasks["lint"].cmd(), "cargo clippy");
        assert_eq!(base.tasks["test"].cmd(), "cargo test");
        assert_eq!(base.lanes["ci"].steps.len(), 1);
        assert!(base.lanes.contains_key("deploy"));
    }

    #[test]
    fn create_lane_repo_scaffolds_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        create_lane_repo("my-lanes", tmp.path(), Some("Test lanes"), true, false).unwrap();

        let target = tmp.path().join("my-lanes");
        assert!(target.join("fledge.toml").exists());
        assert!(target.join("README.md").exists());
        assert!(target.join(".gitignore").exists());

        let content = std::fs::read_to_string(target.join("fledge.toml")).unwrap();
        let parsed: FledgeFileWithLanes = toml::from_str(&content).unwrap();
        assert!(!parsed.lanes.is_empty());
        assert!(!parsed.tasks.is_empty());
    }

    #[test]
    fn create_lane_repo_fails_if_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("existing")).unwrap();
        let result = create_lane_repo("existing", tmp.path(), None, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn validate_valid_lanes() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("fledge.toml"),
            r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"

[lanes.ci]
description = "CI pipeline"
steps = ["lint", "test"]
"#,
        )
        .unwrap();

        let result = validate_lanes(tmp.path(), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_undefined_task_ref() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("fledge.toml"),
            r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
description = "CI"
steps = ["lint", "nonexistent"]
"#,
        )
        .unwrap();

        let result = validate_lanes(tmp.path(), false, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_empty_steps() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("fledge.toml"),
            r#"
[lanes.empty]
description = "Empty"
steps = []
"#,
        )
        .unwrap();

        let result = validate_lanes(tmp.path(), false, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_missing_description_warns() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("fledge.toml"),
            r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
steps = ["lint"]
"#,
        )
        .unwrap();

        // non-strict: passes with warnings
        let result = validate_lanes(tmp.path(), false, false);
        assert!(result.is_ok());

        // strict: fails on warnings
        let result = validate_lanes(tmp.path(), true, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_no_lanes_is_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("fledge.toml"),
            r#"
[tasks]
lint = "cargo clippy"
"#,
        )
        .unwrap();

        let result = validate_lanes(tmp.path(), false, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_json_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("fledge.toml"),
            r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
description = "CI"
steps = ["lint"]
"#,
        )
        .unwrap();

        let result = validate_lanes(tmp.path(), false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn imported_lanes_get_source_tracked() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fledge_toml = tmp.path().join("fledge.toml");
        std::fs::write(
            &fledge_toml,
            "[tasks]\nlint = \"echo lint\"\n\n[lanes.local]\nsteps = [\"lint\"]\n",
        )
        .unwrap();

        let lanes_dir = tmp.path().join(".fledge").join("lanes");
        std::fs::create_dir_all(&lanes_dir).unwrap();
        std::fs::write(
            lanes_dir.join("corvidlabs-fledge-lanes.toml"),
            "# Imported from CorvidLabs/fledge-lanes\n\n[tasks]\ntest = \"echo test\"\n\n[lanes.ci]\ndescription = \"CI\"\nsteps = [\"lint\", \"test\"]\n",
        )
        .unwrap();
        std::fs::write(
            lanes_dir.join("someuser-lanes.toml"),
            "# Imported from someuser/lanes\n\n[lanes.deploy]\nsteps = [{ run = \"echo deploy\" }]\n",
        )
        .unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let config = load_lane_config().unwrap();
        std::env::set_current_dir(&prev).unwrap();

        assert!(config.lanes["local"].source.is_none());

        let ci_source = config.lanes["ci"].source.as_deref();
        assert_eq!(ci_source, Some("CorvidLabs/fledge-lanes"));
        assert_eq!(
            determine_trust_tier(ci_source.unwrap()),
            crate::trust::TrustTier::Official
        );

        let deploy_source = config.lanes["deploy"].source.as_deref();
        assert_eq!(deploy_source, Some("someuser/lanes"));
        assert_eq!(
            determine_trust_tier(deploy_source.unwrap()),
            crate::trust::TrustTier::Unverified
        );
    }

    #[test]
    fn list_lanes_json_includes_trust_tier() {
        let mut lanes = BTreeMap::new();
        lanes.insert(
            "local".to_string(),
            LaneDef {
                description: Some("Local lane".to_string()),
                steps: vec![Step::TaskRef("lint".to_string())],
                fail_fast: true,
                source: None,
            },
        );
        lanes.insert(
            "imported".to_string(),
            LaneDef {
                description: Some("Remote lane".to_string()),
                steps: vec![Step::TaskRef("test".to_string())],
                fail_fast: true,
                source: Some("CorvidLabs/fledge-lanes".to_string()),
            },
        );
        lanes.insert(
            "third_party".to_string(),
            LaneDef {
                description: Some("Third party".to_string()),
                steps: vec![Step::TaskRef("deploy".to_string())],
                fail_fast: true,
                source: Some("someuser/lanes".to_string()),
            },
        );

        let result = list_lanes(&lanes, true);
        assert!(result.is_ok());
    }
}
