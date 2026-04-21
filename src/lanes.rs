use anyhow::{Context, Result, bail};
use console::style;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::run::detect_project_type;

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
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Step {
    TaskRef(String),
    Inline { run: String },
    Parallel { parallel: Vec<String> },
}

pub enum LaneAction {
    Run { name: String, dry_run: bool },
    List { json: bool },
    Init,
    Search { query: Option<String>, json: bool },
    Import { source: String },
}

pub fn run(action: LaneAction) -> Result<()> {
    match action {
        LaneAction::Search { query, json } => search_lanes(query.as_deref(), json),
        LaneAction::Import { source } => import_lanes(&source),
        LaneAction::Init => init_lanes(),
        LaneAction::List { json } => {
            let config = load_lane_config()?;
            list_lanes(&config.lanes, json)
        }
        LaneAction::Run { name, dry_run } => {
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
                execute_lane(&name, lane, &config.tasks, &project_dir)
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
    let config: FledgeFileWithLanes = toml::from_str(&content).context("parsing fledge.toml")?;

    if config.lanes.is_empty() {
        bail!(
            "No lanes defined in fledge.toml.\n  Add a [lanes] section or run {} to add defaults.",
            style("fledge lane init").cyan()
        );
    }

    Ok(config)
}

fn list_lanes(lanes: &BTreeMap<String, LaneDef>, json: bool) -> Result<()> {
    if json {
        let entries: Vec<serde_json::Value> = lanes
            .iter()
            .map(|(name, lane)| {
                serde_json::json!({
                    "name": name,
                    "description": lane.description,
                    "steps": lane.steps.len(),
                    "fail_fast": lane.fail_fast,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    println!("{}", style("Available lanes:").bold());
    let max_name_len = lanes.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, lane) in lanes {
        let desc = lane.description.as_deref().unwrap_or("(no description)");
        println!(
            "  {:<width$}  {}",
            style(name).green(),
            style(desc).dim(),
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
                for name in parallel {
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
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(parallel.join(", ")).cyan(),
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
) -> Result<()> {
    let desc = lane.description.as_deref().unwrap_or("(no description)");
    println!(
        "{} {} — {}",
        style("▶️ Lane:").cyan().bold(),
        style(lane_name).bold(),
        style(desc).dim()
    );

    let total_steps = lane.steps.len();
    let mut failures: Vec<String> = Vec::new();

    for (i, step) in lane.steps.iter().enumerate() {
        let result = match step {
            Step::TaskRef(name) => execute_task_with_deps(name, tasks, project_dir),
            Step::Inline { run: cmd } => execute_inline(cmd, project_dir),
            Step::Parallel { parallel } => execute_parallel(parallel, tasks, project_dir),
        };

        if let Err(e) = result {
            let step_desc = match step {
                Step::TaskRef(name) => name.clone(),
                Step::Inline { run: cmd } => cmd.clone(),
                Step::Parallel { parallel } => format!("parallel({})", parallel.join(", ")),
            };
            if lane.fail_fast {
                bail!(
                    "Lane '{}' failed at step {} ({}): {}",
                    lane_name,
                    i + 1,
                    step_desc,
                    e
                );
            }
            eprintln!(
                "  {} Step {} ({}) failed: {}",
                style("❌").red().bold(),
                i + 1,
                step_desc,
                e
            );
            failures.push(step_desc);
        }
    }

    if failures.is_empty() {
        println!(
            "{} Lane {} completed ({} steps)",
            style("✅").green().bold(),
            style(lane_name).green(),
            total_steps
        );
    } else {
        bail!(
            "Lane '{}' completed with {} failure(s): {}",
            lane_name,
            failures.len(),
            failures.join(", ")
        );
    }

    Ok(())
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
    task_names: &[String],
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
) -> Result<()> {
    let names_display = task_names.join(", ");
    println!(
        "  {} {}",
        style("▶️").cyan().bold(),
        style(format!("Running parallel: {names_display}")).bold()
    );

    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|s| {
        let mut handles = Vec::new();

        for name in task_names {
            let errors = Arc::clone(&errors);
            let handle = s.spawn(move || {
                if let Err(e) = execute_task_with_deps(name, tasks, project_dir) {
                    if let Ok(mut errs) = errors.lock() {
                        errs.push(format!("{}: {}", name, e));
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
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
steps = ["fmt", "lint", "typecheck", "test"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
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

fn init_lanes() -> Result<()> {
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

    println!(
        "{} Added default lanes to {}",
        style("✅").green().bold(),
        style("fledge.toml").cyan()
    );
    println!("  Run {} to see them.", style("fledge lane").cyan());
    Ok(())
}

fn search_lanes(keyword: Option<&str>, json: bool) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let query = match keyword {
        Some(kw) => format!("{} topic:fledge-lane", kw),
        None => "topic:fledge-lane".to_string(),
    };

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
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    println!("{}\n", style("Community lanes on GitHub:").bold());
    let max_name = results
        .iter()
        .map(|r| r.full_name().len())
        .max()
        .unwrap_or(0);
    for r in &results {
        let stars = crate::search::format_stars(r.stars);
        let desc = if r.description.len() > 60 {
            format!("{}...", &r.description[..57])
        } else {
            r.description.clone()
        };
        println!(
            "  {:<width$}  {}  {}",
            style(&r.full_name()).green(),
            style(format!("(⭐ {})", stars)).dim(),
            style(&desc).dim(),
            width = max_name,
        );
    }
    println!(
        "\n{}",
        style("Import with: fledge lane import <owner/repo[/path]>").dim()
    );

    Ok(())
}

fn import_lanes(source: &str) -> Result<()> {
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

    let sp = crate::spinner::Spinner::start(&format!("Fetching lanes from {}:", display_source,));

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

    sp.finish();

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

    let local_content =
        std::fs::read_to_string(&local_path).context("reading local fledge.toml")?;
    let local_config: FledgeFileWithLanes =
        toml::from_str(&local_content).context("parsing local fledge.toml")?;

    let mut imported_lanes = Vec::new();
    let mut imported_tasks = Vec::new();
    let mut skipped = Vec::new();
    let mut append = String::new();

    for (task_name, task_def) in &remote_config.tasks {
        if local_config.tasks.contains_key(task_name) {
            continue;
        }
        let cmd = task_def.cmd();
        append.push_str(&format!("\n[tasks.{task_name}]\ncmd = \"{cmd}\"\n"));
        imported_tasks.push(task_name.clone());
    }

    for (lane_name, lane) in &remote_config.lanes {
        if local_config.lanes.contains_key(lane_name) {
            skipped.push(lane_name.clone());
            continue;
        }
        append.push_str(&format_lane_toml(lane_name, lane));
        imported_lanes.push(lane_name.clone());
    }

    if imported_lanes.is_empty() {
        println!(
            "{} All lanes from {} already exist locally ({})",
            style("*").cyan().bold(),
            display_source,
            skipped.join(", ")
        );
        return Ok(());
    }

    let new_content = format!("{}{}", local_content.trim_end(), append);
    std::fs::write(&local_path, new_content).context("writing fledge.toml")?;

    println!(
        "{} Imported {} lane(s) from {}",
        style("✅").green().bold(),
        imported_lanes.len(),
        display_source
    );
    for name in &imported_lanes {
        println!("  {} {}", style("+").green(), style(name).cyan());
    }
    if !imported_tasks.is_empty() {
        println!(
            "  {} Also added {} task(s): {}",
            style("+").green(),
            imported_tasks.len(),
            imported_tasks.join(", ")
        );
    }
    if !skipped.is_empty() {
        println!(
            "  {} Skipped (already exist): {}",
            style("*").dim(),
            skipped.join(", ")
        );
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

fn format_lane_toml(name: &str, lane: &LaneDef) -> String {
    let mut out = format!("\n[lanes.{}]\n", name);
    if let Some(ref desc) = lane.description {
        out.push_str(&format!("description = \"{}\"\n", desc));
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
                out.push_str(&format!("\"{}\"", name));
            }
            Step::Inline { run: cmd } => {
                out.push_str(&format!("{{ run = \"{}\" }}", cmd));
            }
            Step::Parallel { parallel } => {
                let names: Vec<String> = parallel.iter().map(|n| format!("\"{}\"", n)).collect();
                out.push_str(&format!("{{ parallel = [{}] }}", names.join(", ")));
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
                assert_eq!(parallel, &["lint", "fmt"]);
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
        let result = execute_lane("seq", &config.lanes["seq"], &config.tasks, &project_dir);
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
        let result = execute_lane("par", &config.lanes["par"], &config.tasks, &project_dir);
        assert!(result.is_ok());
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
        let result = execute_lane("ff", &config.lanes["ff"], &config.tasks, &project_dir);
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
        let result = execute_lane("noff", &config.lanes["noff"], &config.tasks, &project_dir);
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
        let result = execute_lane("ci", &config.lanes["ci"], &config.tasks, &project_dir);
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
        };
        let toml = format_lane_toml("test", &lane);
        assert!(toml.contains("{ run = \"echo hello\" }"));
    }

    #[test]
    fn format_lane_toml_with_parallel() {
        let lane = LaneDef {
            description: None,
            steps: vec![Step::Parallel {
                parallel: vec!["lint".to_string(), "fmt".to_string()],
            }],
            fail_fast: true,
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
}
