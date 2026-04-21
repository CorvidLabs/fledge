use anyhow::{Context, Result, bail};
use console::style;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::run::detect_project_type;

#[derive(Debug, Deserialize)]
struct FledgeFileWithFlows {
    #[serde(default)]
    tasks: BTreeMap<String, TaskDef>,
    #[serde(default)]
    flows: BTreeMap<String, FlowDef>,
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
pub struct FlowDef {
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

pub enum FlowAction {
    Run { name: String, dry_run: bool },
    List { json: bool },
    Init,
    Search { query: Option<String>, json: bool },
    Import { source: String },
}

pub fn run(action: FlowAction) -> Result<()> {
    match action {
        FlowAction::Search { query, json } => search_flows(query.as_deref(), json),
        FlowAction::Import { source } => import_flows(&source),
        FlowAction::Init => init_flows(),
        FlowAction::List { json } => {
            let config = load_flow_config()?;
            list_flows(&config.flows, json)
        }
        FlowAction::Run { name, dry_run } => {
            let config = load_flow_config()?;
            let flow = config.flows.get(&name).ok_or_else(|| {
                let available: Vec<&str> = config.flows.keys().map(|s| s.as_str()).collect();
                anyhow::anyhow!(
                    "Unknown flow '{}'. Available flows: {}",
                    name,
                    available.join(", ")
                )
            })?;

            if flow.steps.is_empty() {
                bail!("Flow '{}' has no steps defined", name);
            }

            validate_flow(&name, flow, &config.tasks)?;

            if dry_run {
                dry_run_flow(&name, flow)
            } else {
                let project_dir = std::env::current_dir().context("getting current directory")?;
                execute_flow(&name, flow, &config.tasks, &project_dir)
            }
        }
    }
}

fn load_flow_config() -> Result<FledgeFileWithFlows> {
    let project_dir = std::env::current_dir().context("getting current directory")?;
    let config_path = project_dir.join("fledge.toml");

    if !config_path.exists() {
        bail!(
            "No fledge.toml found in current directory.\n  Run {} to create one.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&config_path).context("reading fledge.toml")?;
    let config: FledgeFileWithFlows = toml::from_str(&content).context("parsing fledge.toml")?;

    if config.flows.is_empty() {
        bail!(
            "No flows defined in fledge.toml.\n  Add a [flows] section or run {} to add defaults.",
            style("fledge flow init").cyan()
        );
    }

    Ok(config)
}

fn list_flows(flows: &BTreeMap<String, FlowDef>, json: bool) -> Result<()> {
    if json {
        let entries: Vec<serde_json::Value> = flows
            .iter()
            .map(|(name, flow)| {
                serde_json::json!({
                    "name": name,
                    "description": flow.description,
                    "steps": flow.steps.len(),
                    "fail_fast": flow.fail_fast,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    println!("{}", style("Available flows:").bold());
    let max_name_len = flows.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, flow) in flows {
        let desc = flow.description.as_deref().unwrap_or("(no description)");
        println!(
            "  {:<width$}  {}",
            style(name).green(),
            style(desc).dim(),
            width = max_name_len
        );
    }
    Ok(())
}

fn validate_flow(flow_name: &str, flow: &FlowDef, tasks: &BTreeMap<String, TaskDef>) -> Result<()> {
    for (i, step) in flow.steps.iter().enumerate() {
        match step {
            Step::TaskRef(name) => {
                if !tasks.contains_key(name) {
                    bail!(
                        "Flow '{}' step {} references unknown task '{}'.\n  Define it in [tasks] first.",
                        flow_name,
                        i + 1,
                        name
                    );
                }
            }
            Step::Inline { .. } => {}
            Step::Parallel { parallel } => {
                for name in parallel {
                    if !tasks.contains_key(name) {
                        bail!(
                            "Flow '{}' step {} parallel group references unknown task '{}'.\n  Define it in [tasks] first.",
                            flow_name,
                            i + 1,
                            name
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn dry_run_flow(flow_name: &str, flow: &FlowDef) -> Result<()> {
    let desc = flow.description.as_deref().unwrap_or("(no description)");
    println!(
        "{} {} — {}",
        style("Flow:").bold(),
        style(flow_name).green(),
        style(desc).dim()
    );
    if !flow.fail_fast {
        println!("  {} fail_fast = false", style("⚙").dim());
    }
    for (i, step) in flow.steps.iter().enumerate() {
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

fn execute_flow(
    flow_name: &str,
    flow: &FlowDef,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
) -> Result<()> {
    let desc = flow.description.as_deref().unwrap_or("(no description)");
    println!(
        "{} {} — {}",
        style("▶️ Flow:").cyan().bold(),
        style(flow_name).bold(),
        style(desc).dim()
    );

    let total_steps = flow.steps.len();
    let mut failures: Vec<String> = Vec::new();

    for (i, step) in flow.steps.iter().enumerate() {
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
            if flow.fail_fast {
                bail!(
                    "Flow '{}' failed at step {} ({}): {}",
                    flow_name,
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
            "{} Flow {} completed ({} steps)",
            style("✅").green().bold(),
            style(flow_name).green(),
            total_steps
        );
    } else {
        bail!(
            "Flow '{}' completed with {} failure(s): {}",
            flow_name,
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
    let task = tasks
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", name))?;

    for dep in task.deps() {
        execute_task_with_deps(dep, tasks, project_dir)?;
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
                    let mut errs = errors.lock().unwrap();
                    errs.push(format!("{}: {}", name, e));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    let errs = errors.lock().unwrap();
    if !errs.is_empty() {
        bail!("Parallel step failed:\n  {}", errs.join("\n  "));
    }

    Ok(())
}

fn flow_defaults(project_type: &str) -> &'static str {
    match project_type {
        "rust" => {
            r#"
[flows.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[flows.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "node" => {
            r#"
[flows.ci]
description = "Run full CI pipeline"
steps = ["lint", "test", "build"]

[flows.check]
description = "Quick quality check"
steps = [
  { parallel = ["lint", "test"] },
]
"#
        }
        "go" => {
            r#"
[flows.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[flows.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "python" => {
            r#"
[flows.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "typecheck", "test"]

[flows.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        _ => {
            r#"
[flows.ci]
description = "Run full CI pipeline"
steps = ["lint", "test", "build"]
"#
        }
    }
}

fn init_flows() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = cwd.join("fledge.toml");

    if !path.exists() {
        bail!(
            "No fledge.toml found. Run {} first, then add flows.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&path).context("reading fledge.toml")?;

    if content.contains("[flows") {
        bail!("Flows already defined in fledge.toml. Edit them manually.");
    }

    let project_type = detect_project_type(&cwd);
    let defaults = flow_defaults(project_type);

    let new_content = format!("{}{}", content.trim_end(), defaults);
    std::fs::write(&path, new_content).context("writing fledge.toml")?;

    println!(
        "{} Added default flows to {}",
        style("✅").green().bold(),
        style("fledge.toml").cyan()
    );
    println!("  Run {} to see them.", style("fledge flow").cyan());
    Ok(())
}

fn search_flows(keyword: Option<&str>, json: bool) -> Result<()> {
    let config = crate::config::Config::load()?;
    let token = config.github_token();

    let query = match keyword {
        Some(kw) => format!("{} topic:fledge-flow", kw),
        None => "topic:fledge-flow".to_string(),
    };

    let sp = crate::spinner::Spinner::start("Searching GitHub for community flows...");

    let body = crate::github::github_api_get(
        "/search/repositories",
        token.as_deref(),
        &[("q", &query), ("sort", "stars"), ("per_page", "30")],
    )
    .context("searching GitHub for flow repos")?;

    sp.finish();

    let results = crate::search::parse_search_response(&body)?;

    if results.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "{} No community flows found{}.",
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

    println!("{}\n", style("Community flows on GitHub:").bold());
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
        style("Import with: fledge flow import <owner/repo[/path]>").dim()
    );

    Ok(())
}

fn import_flows(source: &str) -> Result<()> {
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

    let sp = crate::spinner::Spinner::start(&format!(
        "Fetching flows from {}...",
        display_source,
    ));

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

    let remote_config: FledgeFileWithFlows =
        toml::from_str(&remote_content).context("parsing remote fledge.toml")?;

    if remote_config.flows.is_empty() {
        bail!("Remote repo has no [flows] defined in fledge.toml.");
    }

    let local_content =
        std::fs::read_to_string(&local_path).context("reading local fledge.toml")?;
    let local_config: FledgeFileWithFlows =
        toml::from_str(&local_content).context("parsing local fledge.toml")?;

    let mut imported_flows = Vec::new();
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

    for (flow_name, flow) in &remote_config.flows {
        if local_config.flows.contains_key(flow_name) {
            skipped.push(flow_name.clone());
            continue;
        }
        append.push_str(&format_flow_toml(flow_name, flow));
        imported_flows.push(flow_name.clone());
    }

    if imported_flows.is_empty() {
        println!(
            "{} All flows from {} already exist locally ({})",
            style("*").cyan().bold(),
            display_source,
            skipped.join(", ")
        );
        return Ok(());
    }

    let new_content = format!("{}{}", local_content.trim_end(), append);
    std::fs::write(&local_path, new_content).context("writing fledge.toml")?;

    println!(
        "{} Imported {} flow(s) from {}",
        style("✅").green().bold(),
        imported_flows.len(),
        display_source
    );
    for name in &imported_flows {
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

fn format_flow_toml(name: &str, flow: &FlowDef) -> String {
    let mut out = format!("\n[flows.{}]\n", name);
    if let Some(ref desc) = flow.description {
        out.push_str(&format!("description = \"{}\"\n", desc));
    }
    if !flow.fail_fast {
        out.push_str("fail_fast = false\n");
    }
    out.push_str("steps = [");
    for (i, step) in flow.steps.iter().enumerate() {
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

    fn parse_config(toml_str: &str) -> FledgeFileWithFlows {
        toml::from_str(toml_str).unwrap()
    }

    #[test]
    fn parse_sequential_flow() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[flows.ci]
description = "CI pipeline"
steps = ["lint", "test", "build"]
"#,
        );
        assert_eq!(config.flows.len(), 1);
        assert_eq!(config.flows["ci"].steps.len(), 3);
        assert!(config.flows["ci"].fail_fast);
    }

    #[test]
    fn parse_inline_step() {
        let config = parse_config(
            r#"
[tasks]
test = "cargo test"

[flows.release]
description = "Release"
steps = [
  "test",
  { run = "cargo build --release" },
]
"#,
        );
        assert_eq!(config.flows["release"].steps.len(), 2);
        match &config.flows["release"].steps[1] {
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

[flows.check]
description = "Quick check"
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]
"#,
        );
        assert_eq!(config.flows["check"].steps.len(), 2);
        match &config.flows["check"].steps[0] {
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

[flows.audit]
description = "Audit"
fail_fast = false
steps = ["a", "b"]
"#,
        );
        assert!(!config.flows["audit"].fail_fast);
    }

    #[test]
    fn parse_fail_fast_default_true() {
        let config = parse_config(
            r#"
[tasks]
a = "echo a"

[flows.ci]
steps = ["a"]
"#,
        );
        assert!(config.flows["ci"].fail_fast);
    }

    #[test]
    fn validate_unknown_task_ref() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"

[flows.ci]
steps = ["lint", "nonexistent"]
"#,
        );
        let result = validate_flow("ci", &config.flows["ci"], &config.tasks);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn validate_unknown_parallel_ref() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"

[flows.check]
steps = [{ parallel = ["lint", "ghost"] }]
"#,
        );
        let result = validate_flow("check", &config.flows["check"], &config.tasks);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ghost"));
    }

    #[test]
    fn validate_inline_always_ok() {
        let config = parse_config(
            r#"
[tasks]

[flows.ci]
steps = [{ run = "echo hello" }]
"#,
        );
        let result = validate_flow("ci", &config.flows["ci"], &config.tasks);
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

[flows.ci]
steps = ["lint", "test", "build"]
"#,
        );
        let result = validate_flow("ci", &config.flows["ci"], &config.tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_multiple_flows() {
        let config = parse_config(
            r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[flows.ci]
description = "CI"
steps = ["lint", "test", "build"]

[flows.quick]
description = "Quick"
steps = ["lint"]
"#,
        );
        assert_eq!(config.flows.len(), 2);
        assert!(config.flows.contains_key("ci"));
        assert!(config.flows.contains_key("quick"));
    }

    #[test]
    fn parse_no_flows_section() {
        let config = parse_config(
            r#"
[tasks]
build = "cargo build"
"#,
        );
        assert!(config.flows.is_empty());
    }

    #[test]
    fn parse_empty_flows_section() {
        let config = parse_config(
            r#"
[tasks]
build = "cargo build"

[flows]
"#,
        );
        assert!(config.flows.is_empty());
    }

    #[test]
    fn parse_mixed_step_types() {
        let config = parse_config(
            r#"
[tasks]
test = "cargo test"
lint = "cargo clippy"

[flows.full]
steps = [
  "test",
  { run = "echo done" },
  { parallel = ["test", "lint"] },
]
"#,
        );
        assert_eq!(config.flows["full"].steps.len(), 3);
        assert!(matches!(&config.flows["full"].steps[0], Step::TaskRef(_)));
        assert!(matches!(
            &config.flows["full"].steps[1],
            Step::Inline { .. }
        ));
        assert!(matches!(
            &config.flows["full"].steps[2],
            Step::Parallel { .. }
        ));
    }

    #[test]
    fn execute_sequential_flow_echo() {
        let config = parse_config(
            r#"
[tasks]
a = "echo step-a"
b = "echo step-b"

[flows.seq]
description = "Sequential"
steps = ["a", "b"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_flow("seq", &config.flows["seq"], &config.tasks, &project_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_inline_step() {
        let config = parse_config(
            r#"
[tasks]

[flows.inline]
steps = [{ run = "echo inline-works" }]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_flow(
            "inline",
            &config.flows["inline"],
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

[flows.par]
steps = [{ parallel = ["a", "b"] }]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_flow("par", &config.flows["par"], &config.tasks, &project_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_fail_fast_stops() {
        let config = parse_config(
            r#"
[tasks]
fail = "exit 1"
ok = "echo ok"

[flows.ff]
fail_fast = true
steps = ["fail", "ok"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_flow("ff", &config.flows["ff"], &config.tasks, &project_dir);
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

[flows.noff]
fail_fast = false
steps = ["fail", "ok"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_flow("noff", &config.flows["noff"], &config.tasks, &project_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("1 failure"));
    }

    #[test]
    fn execute_task_deps_in_flow() {
        let config = parse_config(
            r#"
[tasks.build]
cmd = "echo building"
deps = ["prep"]

[tasks.prep]
cmd = "echo preparing"

[flows.ci]
steps = ["build"]
"#,
        );
        let project_dir = std::env::current_dir().unwrap();
        let result = execute_flow("ci", &config.flows["ci"], &config.tasks, &project_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn flow_defaults_are_valid_toml() {
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
            let defaults = flow_defaults(project_type);
            let toml_str = format!("{}{}", tasks, defaults);
            let result: Result<FledgeFileWithFlows, _> = toml::from_str(&toml_str);
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
        let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-flows");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-flows");
        assert!(subpath.is_none());
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_import_source_with_ref() {
        let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-flows@v1.0.0");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-flows");
        assert!(subpath.is_none());
        assert_eq!(git_ref.unwrap(), "v1.0.0");
    }

    #[test]
    fn parse_import_source_with_subpath() {
        let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-flows/rust");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-flows");
        assert_eq!(subpath.unwrap(), "rust");
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_import_source_with_subpath_and_ref() {
        let (owner, repo, subpath, git_ref) =
            parse_import_source("CorvidLabs/fledge-flows/rust@main");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-flows");
        assert_eq!(subpath.unwrap(), "rust");
        assert_eq!(git_ref.unwrap(), "main");
    }

    #[test]
    fn parse_import_source_full_url() {
        let (owner, repo, subpath, git_ref) =
            parse_import_source("https://github.com/CorvidLabs/fledge-flows.git");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-flows");
        assert!(subpath.is_none());
        assert!(git_ref.is_none());
    }

    #[test]
    fn parse_import_source_url_with_ref() {
        let (owner, repo, subpath, git_ref) =
            parse_import_source("https://github.com/CorvidLabs/fledge-flows@main");
        assert_eq!(owner, "CorvidLabs");
        assert_eq!(repo, "fledge-flows");
        assert!(subpath.is_none());
        assert_eq!(git_ref.unwrap(), "main");
    }

    #[test]
    fn format_flow_toml_sequential() {
        let flow = FlowDef {
            description: Some("CI pipeline".to_string()),
            steps: vec![
                Step::TaskRef("lint".to_string()),
                Step::TaskRef("test".to_string()),
            ],
            fail_fast: true,
        };
        let toml = format_flow_toml("ci", &flow);
        assert!(toml.contains("[flows.ci]"));
        assert!(toml.contains("description = \"CI pipeline\""));
        assert!(toml.contains("\"lint\""));
        assert!(toml.contains("\"test\""));
        assert!(!toml.contains("fail_fast"));
    }

    #[test]
    fn format_flow_toml_with_fail_fast_false() {
        let flow = FlowDef {
            description: None,
            steps: vec![Step::TaskRef("audit".to_string())],
            fail_fast: false,
        };
        let toml = format_flow_toml("audit", &flow);
        assert!(toml.contains("fail_fast = false"));
    }

    #[test]
    fn format_flow_toml_with_inline() {
        let flow = FlowDef {
            description: None,
            steps: vec![Step::Inline {
                run: "echo hello".to_string(),
            }],
            fail_fast: true,
        };
        let toml = format_flow_toml("test", &flow);
        assert!(toml.contains("{ run = \"echo hello\" }"));
    }

    #[test]
    fn format_flow_toml_with_parallel() {
        let flow = FlowDef {
            description: None,
            steps: vec![Step::Parallel {
                parallel: vec!["lint".to_string(), "fmt".to_string()],
            }],
            fail_fast: true,
        };
        let toml = format_flow_toml("check", &flow);
        assert!(toml.contains("parallel"));
        assert!(toml.contains("\"lint\""));
        assert!(toml.contains("\"fmt\""));
    }

    #[test]
    fn format_flow_toml_roundtrips() {
        let flow = FlowDef {
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
            format_flow_toml("ci", &flow)
        );
        let parsed: FledgeFileWithFlows = toml::from_str(&toml_str).unwrap();
        assert!(parsed.flows.contains_key("ci"));
        assert_eq!(parsed.flows["ci"].steps.len(), 3);
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
