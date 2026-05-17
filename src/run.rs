use anyhow::{bail, Context, Result};
use console::style;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::process::Command;

/// Per-command JSON schema versions for `run` subcommands. See lanes.rs for
/// rationale. (Note: this is the wire-envelope version, distinct from the
/// `schema_version` field on `fledge.toml` itself, which is a manifest version.)
const RUN_LIST_SCHEMA: u32 = 1;
const RUN_TASK_SCHEMA: u32 = 1;
const RUN_INIT_SCHEMA: u32 = 1;

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
    pub lang: Option<String>,
    pub json: bool,
}

#[derive(Serialize)]
struct TaskInfo {
    name: String,
    cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    deps: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    env: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dir: Option<String>,
}

pub fn run(opts: RunOptions) -> Result<()> {
    if opts.init {
        return init_fledge_toml(opts.lang.as_deref(), opts.json);
    }

    let project_dir = std::env::current_dir().context("getting current directory")?;
    let config_path = project_dir.join("fledge.toml");

    let (tasks, is_auto) = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).context("reading fledge.toml")?;
        let config: FledgeFile = toml::from_str(&content).context("parsing fledge.toml")?;
        if config.tasks.is_empty() {
            bail!(
                "No tasks defined in fledge.toml.\n  Add a [tasks] section with task definitions."
            );
        }
        (config.tasks, false)
    } else {
        let project_type = detect_project_type(&project_dir);
        if project_type == "generic" {
            bail!(
                "Could not detect project type and no fledge.toml found.\n  Run {} to create one.",
                style("fledge run --init").cyan()
            );
        }
        let defaults = auto_detect_tasks(project_type, &project_dir);
        (defaults, true)
    };

    if opts.list || opts.task.is_none() {
        if opts.json {
            return list_tasks_json(&tasks, is_auto);
        }
        if is_auto {
            println!(
                "{} Auto-detected tasks (create {} to customize)\n",
                style("*").cyan().bold(),
                style("fledge.toml").cyan()
            );
        }
        return list_tasks(&tasks);
    }

    let task_name = opts.task.as_ref().unwrap();
    if !tasks.contains_key(task_name) {
        let available: Vec<&str> = tasks.keys().map(|s| s.as_str()).collect();
        bail!(
            "Unknown task '{}'. Available tasks: {}",
            task_name,
            available.join(", ")
        );
    }

    if is_auto {
        println!(
            "{} Running auto-detected task (no fledge.toml)\n",
            style("*").cyan().bold(),
        );
    }

    let mut visited = HashSet::new();
    execute_task(task_name, &tasks, &project_dir, &mut visited, opts.json)
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

fn list_tasks_json(tasks: &BTreeMap<String, TaskDef>, auto_detected: bool) -> Result<()> {
    let task_list: Vec<TaskInfo> = tasks
        .iter()
        .map(|(name, task)| TaskInfo {
            name: name.clone(),
            cmd: task.cmd().to_string(),
            description: task.description().map(|s| s.to_string()),
            deps: task.deps().to_vec(),
            env: task.env().clone(),
            dir: task.dir().map(|s| s.to_string()),
        })
        .collect();
    let envelope = serde_json::json!({
        "schema_version": RUN_LIST_SCHEMA,
        "action": "run_list",
        "auto_detected": auto_detected,
        "tasks": task_list,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

fn execute_task(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    visited: &mut HashSet<String>,
    json: bool,
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
        execute_task(dep, tasks, project_dir, visited, json)?;
    }

    let cmd_str = task.cmd();
    let work_dir = match task.dir() {
        Some(d) => project_dir.join(d),
        None => project_dir.to_path_buf(),
    };

    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    if json {
        let output = Command::new(shell)
            .arg(flag)
            .arg(cmd_str)
            .current_dir(&work_dir)
            .envs(task.env())
            .output()
            .with_context(|| format!("running task '{name}'"))?;

        let result = serde_json::json!({
            "schema_version": RUN_TASK_SCHEMA,
            "action": "run_task",
            "task": name,
            "command": cmd_str,
            "exit_code": output.status.code().unwrap_or(-1),
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);

        if !output.status.success() {
            let code = output.status.code().unwrap_or(1);
            bail!("Task '{}' failed with exit code {}", name, code);
        }
    } else {
        println!(
            "{} {}",
            style("▶️").cyan().bold(),
            style(format!("Running task: {name}")).bold()
        );

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
    }

    Ok(())
}

// Detection order matters for monorepos: first match wins (most specific → least)
pub fn detect_project_type(dir: &Path) -> &'static str {
    if dir.join("Cargo.toml").exists() {
        "rust"
    } else if dir.join("package.json").exists() {
        "node"
    } else if dir.join("go.mod").exists() {
        "go"
    } else if dir.join("pyproject.toml").exists() || dir.join("setup.py").exists() {
        "python"
    } else if dir.join("Gemfile").exists() {
        "ruby"
    } else if dir.join("build.gradle").exists() || dir.join("build.gradle.kts").exists() {
        "java-gradle"
    } else if dir.join("pom.xml").exists() {
        "java-maven"
    } else if dir.join("Package.swift").exists() {
        "swift"
    } else {
        "generic"
    }
}

pub fn task_defaults(project_type: &str, dir: &Path) -> String {
    match project_type {
        "rust" => r#"build = "cargo build"
test = "cargo test"
lint = "cargo clippy -- -D warnings"
fmt = "cargo fmt --check""#
            .to_string(),
        "node" => {
            let runner = detect_node_runner(dir);
            let (run_prefix, test_cmd) = match runner {
                "npm" => ("npm run", "npm test".to_string()),
                other => (other, format!("{other} test")),
            };
            format!(
                r#"build = "{run_prefix} build"
test = "{test_cmd}"
lint = "{run_prefix} lint"
dev = "{run_prefix} dev""#
            )
        }
        "go" => r#"build = "go build ./..."
test = "go test ./..."
lint = "go vet ./..."
fmt = "gofmt -l .""#
            .to_string(),
        "python" => r#"test = "pytest"
lint = "ruff check ."
fmt = "ruff format --check ."
# typecheck = "mypy ."  # uncomment if mypy is installed"#
            .to_string(),
        "ruby" => r#"test = "bundle exec rake test"
lint = "bundle exec rubocop"
console = "bundle exec irb""#
            .to_string(),
        "java-gradle" => {
            let gradlew = if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            };
            format!(
                "build = \"{gradlew} build\"\ntest = \"{gradlew} test\"\nlint = \"{gradlew} check\""
            )
        }
        "java-maven" => r#"build = "mvn compile"
test = "mvn test"
lint = "mvn checkstyle:check""#
            .to_string(),
        "swift" => r#"build = "swift build"
test = "swift test"
# lint = "swiftlint"  # uncomment if swiftlint is installed"#
            .to_string(),
        _ => r#"# build = "make build"
# test = "make test"
# lint = "echo 'add your linter'"#
            .to_string(),
    }
}

pub(crate) fn detect_node_runner(dir: &Path) -> &'static str {
    if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        "bun"
    } else if dir.join("yarn.lock").exists() {
        "yarn"
    } else if dir.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else {
        "npm"
    }
}

fn has_script(dir: &Path, script: &str) -> bool {
    let pkg_path = dir.join("package.json");
    if let Ok(content) = std::fs::read_to_string(pkg_path) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            return parsed.get("scripts").and_then(|s| s.get(script)).is_some();
        }
    }
    false
}

fn auto_detect_tasks(project_type: &str, dir: &Path) -> BTreeMap<String, TaskDef> {
    let mut tasks = BTreeMap::new();

    match project_type {
        "rust" => {
            tasks.insert("build".into(), TaskDef::Short("cargo build".into()));
            tasks.insert("test".into(), TaskDef::Short("cargo test".into()));
            tasks.insert(
                "lint".into(),
                TaskDef::Short("cargo clippy -- -D warnings".into()),
            );
            tasks.insert("fmt".into(), TaskDef::Short("cargo fmt --check".into()));
        }
        "node" => {
            let runner = detect_node_runner(dir);
            let run_prefix = match runner {
                "npm" => "npm run",
                other => other,
            };
            let test_cmd = match runner {
                "npm" => "npm test".to_string(),
                other => format!("{other} test"),
            };

            if has_script(dir, "build") {
                tasks.insert(
                    "build".into(),
                    TaskDef::Short(format!("{run_prefix} build")),
                );
            }
            tasks.insert("test".into(), TaskDef::Short(test_cmd));
            if has_script(dir, "lint") {
                tasks.insert("lint".into(), TaskDef::Short(format!("{run_prefix} lint")));
            }
            if has_script(dir, "dev") {
                tasks.insert("dev".into(), TaskDef::Short(format!("{run_prefix} dev")));
            }
        }
        "go" => {
            tasks.insert("build".into(), TaskDef::Short("go build ./...".into()));
            tasks.insert("test".into(), TaskDef::Short("go test ./...".into()));
            tasks.insert("lint".into(), TaskDef::Short("go vet ./...".into()));
        }
        "python" => {
            tasks.insert("test".into(), TaskDef::Short("pytest".into()));
            tasks.insert("lint".into(), TaskDef::Short("ruff check .".into()));
            tasks.insert("fmt".into(), TaskDef::Short("ruff format --check .".into()));
        }
        "ruby" => {
            tasks.insert(
                "test".into(),
                TaskDef::Short("bundle exec rake test".into()),
            );
            tasks.insert("lint".into(), TaskDef::Short("bundle exec rubocop".into()));
        }
        "java-gradle" => {
            let gradlew = if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            };
            tasks.insert("build".into(), TaskDef::Short(format!("{gradlew} build")));
            tasks.insert("test".into(), TaskDef::Short(format!("{gradlew} test")));
        }
        "java-maven" => {
            tasks.insert("build".into(), TaskDef::Short("mvn compile".into()));
            tasks.insert("test".into(), TaskDef::Short("mvn test".into()));
        }
        _ => {}
    }

    tasks
}

fn init_fledge_toml(lang_override: Option<&str>, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = cwd.join("fledge.toml");
    if path.exists() {
        bail!("fledge.toml already exists in current directory");
    }

    let project_type = lang_override.unwrap_or_else(|| detect_project_type(&cwd));
    let defaults = task_defaults(project_type, &cwd);

    let content = format!(
        r#"# fledge.toml — project task definitions
# Docs: https://github.com/CorvidLabs/fledge#task-runner
# Detected project type: {project_type}

[tasks]
# Simple tasks — just a command string
{defaults}

# Full task with options
# [tasks.ci]
# cmd = "your-test-cmd && your-lint-cmd"
# description = "Run full CI checks"
# deps = ["fmt"]
# env = {{}}
# dir = "."
"#
    );

    std::fs::write(&path, content).context("writing fledge.toml")?;

    if json {
        let envelope = serde_json::json!({
            "schema_version": RUN_INIT_SCHEMA,
            "action": "run_init",
            "file": "fledge.toml",
            "project_type": project_type,
            "files_created": ["fledge.toml"],
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    println!(
        "{} Created {}",
        style("✅").green().bold(),
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
        let project_dir = std::env::temp_dir();
        let mut visited = HashSet::new();
        let result = execute_task("a", &config.tasks, &project_dir, &mut visited, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
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

    #[test]
    fn detect_rust_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "rust");
    }

    #[test]
    fn detect_node_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), "node");
    }

    #[test]
    fn detect_go_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "go");
    }

    #[test]
    fn detect_python_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "python");
    }

    #[test]
    fn detect_python_setup_py() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("setup.py"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "python");
    }

    #[test]
    fn detect_ruby_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Gemfile"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "ruby");
    }

    #[test]
    fn detect_java_gradle_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "java-gradle");
    }

    #[test]
    fn detect_java_gradle_kts_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle.kts"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "java-gradle");
    }

    #[test]
    fn detect_java_maven_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pom.xml"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), "java-maven");
    }

    #[test]
    fn detect_multi_marker_uses_first_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), "rust");
    }

    #[test]
    fn detect_generic_project() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(dir.path()), "generic");
    }

    #[test]
    fn task_defaults_are_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        for project_type in &[
            "rust",
            "node",
            "go",
            "python",
            "ruby",
            "java-gradle",
            "java-maven",
            "generic",
        ] {
            let defaults = task_defaults(project_type, dir.path());
            let toml_str = format!("[tasks]\n{}", defaults);
            let result: Result<FledgeFile, _> = toml::from_str(&toml_str);
            assert!(
                result.is_ok(),
                "Invalid TOML for {}: {:?}",
                project_type,
                result.err()
            );
        }
    }

    #[test]
    fn task_defaults_bun_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bun.lockb"), "").unwrap();
        let defaults = task_defaults("node", dir.path());
        assert!(defaults.contains("bun build"), "should use bun commands");
        assert!(defaults.contains("bun test"), "should use bun test");
        assert!(!defaults.contains("npm"), "should not contain npm");
    }

    #[test]
    fn task_defaults_yarn_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        let defaults = task_defaults("node", dir.path());
        assert!(defaults.contains("yarn build"), "should use yarn commands");
        assert!(defaults.contains("yarn test"), "should use yarn test");
        assert!(!defaults.contains("npm"), "should not contain npm");
    }

    #[test]
    fn auto_detect_rust_tasks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let tasks = auto_detect_tasks("rust", dir.path());
        assert!(tasks.contains_key("build"));
        assert!(tasks.contains_key("test"));
        assert!(tasks.contains_key("lint"));
        assert!(tasks.contains_key("fmt"));
        assert_eq!(tasks["build"].cmd(), "cargo build");
    }

    #[test]
    fn auto_detect_node_npm_tasks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"build":"tsc","test":"jest","lint":"eslint .","dev":"vite"}}"#,
        )
        .unwrap();
        let tasks = auto_detect_tasks("node", dir.path());
        assert_eq!(tasks["build"].cmd(), "npm run build");
        assert_eq!(tasks["test"].cmd(), "npm test");
        assert_eq!(tasks["lint"].cmd(), "npm run lint");
        assert_eq!(tasks["dev"].cmd(), "npm run dev");
    }

    #[test]
    fn auto_detect_node_bun_tasks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"build":"tsc","test":"bun test"}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("bun.lockb"), "").unwrap();
        let tasks = auto_detect_tasks("node", dir.path());
        assert_eq!(tasks["build"].cmd(), "bun build");
        assert_eq!(tasks["test"].cmd(), "bun test");
        assert!(!tasks.contains_key("dev"));
    }

    #[test]
    fn auto_detect_node_yarn_tasks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"build":"tsc","test":"jest"}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        let tasks = auto_detect_tasks("node", dir.path());
        assert_eq!(tasks["build"].cmd(), "yarn build");
        assert_eq!(tasks["test"].cmd(), "yarn test");
    }

    #[test]
    fn auto_detect_node_only_includes_existing_scripts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"test":"jest"}}"#,
        )
        .unwrap();
        let tasks = auto_detect_tasks("node", dir.path());
        assert!(tasks.contains_key("test"));
        assert!(!tasks.contains_key("build"));
        assert!(!tasks.contains_key("lint"));
        assert!(!tasks.contains_key("dev"));
    }

    #[test]
    fn auto_detect_generic_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let tasks = auto_detect_tasks("generic", dir.path());
        assert!(tasks.is_empty());
    }

    #[test]
    fn detect_node_runner_npm_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_node_runner(dir.path()), "npm");
    }

    #[test]
    fn detect_node_runner_bun() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bun.lockb"), "").unwrap();
        assert_eq!(detect_node_runner(dir.path()), "bun");
    }

    #[test]
    fn detect_node_runner_bun_lock() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bun.lock"), "").unwrap();
        assert_eq!(detect_node_runner(dir.path()), "bun");
    }

    #[test]
    fn detect_node_runner_yarn() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(detect_node_runner(dir.path()), "yarn");
    }

    #[test]
    fn detect_node_runner_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_node_runner(dir.path()), "pnpm");
    }
}
