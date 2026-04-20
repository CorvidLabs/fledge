use anyhow::{Context, Result, bail};
use console::style;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct FledgeFile {
    #[serde(default)]
    tasks: BTreeMap<String, TaskDef>,
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
    description: Option<String>,
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

    fn description(&self) -> Option<&str> {
        match self {
            TaskDef::Short(_) => None,
            TaskDef::Full(c) => c.description.as_deref(),
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

pub struct RunOptions {
    pub task: Option<String>,
    pub init: bool,
    pub list: bool,
}

pub fn run(opts: RunOptions) -> Result<()> {
    if opts.init {
        return init_fledge_toml();
    }

    let project_dir = std::env::current_dir().context("getting current directory")?;
    let config_path = project_dir.join("fledge.toml");

    if !config_path.exists() {
        bail!(
            "No fledge.toml found in current directory.\n  Run {} to create one.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&config_path).context("reading fledge.toml")?;
    let config: FledgeFile = toml::from_str(&content).context("parsing fledge.toml")?;

    if config.tasks.is_empty() {
        bail!("No tasks defined in fledge.toml.\n  Add a [tasks] section with task definitions.");
    }

    if opts.list || opts.task.is_none() {
        return list_tasks(&config.tasks);
    }

    let task_name = opts.task.as_ref().unwrap();
    if !config.tasks.contains_key(task_name) {
        let available: Vec<&str> = config.tasks.keys().map(|s| s.as_str()).collect();
        bail!(
            "Unknown task '{}'. Available tasks: {}",
            task_name,
            available.join(", ")
        );
    }

    let mut visited = HashSet::new();
    execute_task(task_name, &config.tasks, &project_dir, &mut visited)
}

fn list_tasks(tasks: &BTreeMap<String, TaskDef>) -> Result<()> {
    println!("{}", style("Available tasks:").bold());
    let max_name_len = tasks.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, task) in tasks {
        let desc = task.description().unwrap_or(task.cmd());
        println!(
            "  {:<width$}  {}",
            style(name).green(),
            style(desc).dim(),
            width = max_name_len
        );
    }
    Ok(())
}

fn execute_task(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    visited: &mut HashSet<String>,
) -> Result<()> {
    if visited.contains(name) {
        bail!(
            "Circular dependency detected: task '{}' depends on itself",
            name
        );
    }
    visited.insert(name.to_string());

    let task = tasks
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found (referenced as dependency)", name))?;

    for dep in task.deps() {
        execute_task(dep, tasks, project_dir, visited)?;
    }

    println!(
        "{} {}",
        style("▸").cyan().bold(),
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

fn init_fledge_toml() -> Result<()> {
    let path = std::env::current_dir()?.join("fledge.toml");
    if path.exists() {
        bail!("fledge.toml already exists in current directory");
    }

    let content = r#"# fledge.toml — project task definitions
# Docs: https://github.com/CorvidLabs/fledge#task-runner

[tasks]
# Simple tasks — just a command string
build = "cargo build"
test = "cargo test"
lint = "cargo clippy -- -D warnings"
fmt = "cargo fmt --check"

# Full task with options
# [tasks.ci]
# cmd = "cargo test && cargo clippy -- -D warnings"
# description = "Run full CI checks"
# deps = ["fmt"]
# env = { RUST_BACKTRACE = "1" }
# dir = "."
"#;

    std::fs::write(&path, content).context("writing fledge.toml")?;
    println!(
        "{} Created {}",
        style("✓").green().bold(),
        style("fledge.toml").cyan()
    );
    println!(
        "  Edit it to define your project tasks, then run {} to see them.",
        style("fledge run").cyan()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_short_tasks() {
        let toml_str = r#"
[tasks]
build = "cargo build"
test = "cargo test"
"#;
        let config: FledgeFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tasks.len(), 2);
        assert_eq!(config.tasks["build"].cmd(), "cargo build");
        assert_eq!(config.tasks["test"].cmd(), "cargo test");
    }

    #[test]
    fn parse_full_tasks() {
        let toml_str = r#"
[tasks.ci]
cmd = "cargo test"
description = "Run CI"
deps = ["lint"]
env = { RUST_BACKTRACE = "1" }

[tasks.lint]
cmd = "cargo clippy"
"#;
        let config: FledgeFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tasks.len(), 2);
        assert_eq!(config.tasks["ci"].cmd(), "cargo test");
        assert_eq!(config.tasks["ci"].deps(), &["lint"]);
        assert_eq!(config.tasks["ci"].description(), Some("Run CI"));
        assert_eq!(
            config.tasks["ci"].env().get("RUST_BACKTRACE"),
            Some(&"1".to_string())
        );
    }

    #[test]
    fn parse_mixed_tasks() {
        let toml_str = r#"
[tasks]
build = "cargo build"

[tasks.deploy]
cmd = "cargo install --path ."
deps = ["build"]
description = "Build and install"
"#;
        let config: FledgeFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tasks.len(), 2);
        assert_eq!(config.tasks["build"].cmd(), "cargo build");
        assert_eq!(config.tasks["deploy"].deps(), &["build"]);
    }

    #[test]
    fn detect_circular_deps() {
        let toml_str = r#"
[tasks.a]
cmd = "echo a"
deps = ["b"]

[tasks.b]
cmd = "echo b"
deps = ["a"]
"#;
        let config: FledgeFile = toml::from_str(toml_str).unwrap();
        let project_dir = std::env::current_dir().unwrap();
        let mut visited = HashSet::new();
        let result = execute_task("a", &config.tasks, &project_dir, &mut visited);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Circular dependency")
        );
    }

    #[test]
    fn empty_tasks_section() {
        let toml_str = r#"
[tasks]
"#;
        let config: FledgeFile = toml::from_str(toml_str).unwrap();
        assert!(config.tasks.is_empty());
    }

    #[test]
    fn parse_with_dir() {
        let toml_str = r#"
[tasks.frontend]
cmd = "npm run build"
dir = "client"
"#;
        let config: FledgeFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tasks["frontend"].dir(), Some("client"));
    }
}
