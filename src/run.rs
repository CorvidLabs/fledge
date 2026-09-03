use anyhow::{bail, Context, Result};
use console::style;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

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
    /// Forward child stdout/stderr live instead of buffering them. Only
    /// affects `--json` runs — the human-readable path already inherits the
    /// terminal, so output there is live either way. See `run_streaming`.
    pub stream: bool,
    /// Pass-through arguments for the target task's command (everything after
    /// `--` on the CLI). Applied to the named task only, never its deps.
    pub args: Vec<String>,
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
    execute_task(
        task_name,
        &tasks,
        &project_dir,
        &mut visited,
        opts.json,
        opts.stream,
        &opts.args,
    )
}

/// Does a shell command reference a positional parameter (`$1`..`$9`, `$@`,
/// `$*`, or their `${...}` braced forms)? When it does, the task author is
/// explicitly placing the pass-through args, so we must NOT also auto-append
/// `"$@"` (which would duplicate them). POSIX-only concept; Windows always
/// appends.
fn references_positional(cmd: &str) -> bool {
    let bytes = cmd.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            // Skip an optional `{` for the `${1}` / `${@}` form.
            let j = if bytes[i + 1] == b'{' { i + 2 } else { i + 1 };
            if let Some(&c) = bytes.get(j) {
                if c.is_ascii_digit() || c == b'@' || c == b'*' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Build the `Command` that runs a task's `cmd` string in a shell, wiring in
/// any pass-through `args` safely.
///
/// On POSIX the args are passed as real positional parameters
/// (`sh -c '<cmd> "$@"' fledge <args...>`) so the shell quotes them — they are
/// never interpolated into the command string, so there is no injection
/// surface. `"$@"` is auto-appended unless the command already references a
/// positional. With no args, the invocation is byte-identical to before this
/// feature existed.
///
/// On Windows (`cmd /C`) there is no `$@`; args are appended as separate argv
/// entries (best-effort — complex quoting is less robust than the POSIX path).
fn build_task_command(
    cmd_str: &str,
    work_dir: &Path,
    env: &BTreeMap<String, String>,
    args: &[String],
) -> Command {
    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    let mut command = Command::new(shell);
    command.arg(flag);

    if cfg!(windows) {
        command.arg(cmd_str);
        command.args(args);
    } else if args.is_empty() {
        command.arg(cmd_str);
    } else if references_positional(cmd_str) {
        // Author placed the args themselves via $1/$@; don't double-append.
        command.arg(cmd_str).arg("fledge").args(args);
    } else {
        // `$0` is set to "fledge" purely so $1.. line up with the user args.
        command
            .arg(format!("{cmd_str} \"$@\""))
            .arg("fledge")
            .args(args);
    }

    command.current_dir(work_dir);
    command.envs(env);
    command
}

/// What a streamed task run produced: the same three pieces `Command::output`
/// yields, except the bytes were also forwarded to the terminal as they
/// arrived rather than only at exit.
struct StreamedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// The first mirror-write failure seen while forwarding, if any. Purely
    /// advisory: the run itself succeeded or failed on the child's own terms,
    /// and `stdout`/`stderr` above are complete regardless.
    mirror_error: Option<io::Error>,
}

/// What one stream's `pump` produced: everything read from the child, plus the
/// first failure (if any) encountered while *mirroring* it.
#[derive(Debug)]
struct PumpOutcome {
    captured: Vec<u8>,
    /// `Some` once a write to the mirror sink failed. Mirroring stops there;
    /// capture continues to EOF.
    mirror_error: Option<io::Error>,
}

/// Copy `reader` to `sink` chunk by chunk, flushing after every chunk, while
/// also accumulating everything read. This is the "tee" that lets `--stream`
/// show output live *and* still fill the JSON envelope.
///
/// Chunks are forwarded verbatim — no colouring, prefixing or line framing —
/// so a non-TTY destination (a pipe, a CI log) receives exactly the child's
/// bytes.
///
/// The two failure modes are deliberately *not* symmetric:
///
/// - A **read** failure on the child's pipe is a real error (`Err`): the
///   capture is now incomplete, so the envelope would lie about the task's
///   output.
/// - A **mirror-write** failure (fledge's own stderr is a closed pipe, a full
///   disk, ...) is recorded and mirroring stops, but capture continues to EOF
///   and the call still returns `Ok`. Failing to *echo* output must never
///   destroy the *result* — the child's real exit status and full output are
///   still owed to the caller.
fn pump<R: Read, W: Write>(mut reader: R, sink: &mut W) -> io::Result<PumpOutcome> {
    let mut captured = Vec::new();
    let mut mirror_error: Option<io::Error> = None;
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                return Ok(PumpOutcome {
                    captured,
                    mirror_error,
                })
            }
            Ok(n) => {
                let chunk = &buf[..n];
                captured.extend_from_slice(chunk);
                if mirror_error.is_none() {
                    // Write-then-flush so a long-running task's partial line
                    // (e.g. a prompt with no trailing newline) reaches the
                    // terminal immediately.
                    if let Err(e) = sink.write_all(chunk).and_then(|()| sink.flush()) {
                        mirror_error = Some(e);
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

/// A forwarding thread's handle: it yields whatever `pump` returned.
type PumpHandle = std::thread::JoinHandle<io::Result<PumpOutcome>>;

/// Join **both** forwarding threads, then propagate the first failure.
///
/// Order matters: joining lazily (`out.join()??` before `err.join()`) would
/// short-circuit on a stdout failure and drop the stderr handle un-joined.
/// Dropping a `JoinHandle` detaches the thread rather than stopping it, so it
/// would keep draining the child's pipe and writing to `io::stderr()` while
/// the CLI unwinds toward exit. Both joins happen unconditionally, and only
/// afterwards does an error escape.
fn join_pumps(
    out_handle: PumpHandle,
    err_handle: PumpHandle,
) -> io::Result<(PumpOutcome, PumpOutcome)> {
    fn collect(
        joined: std::thread::Result<io::Result<PumpOutcome>>,
        which: &str,
    ) -> io::Result<PumpOutcome> {
        joined.unwrap_or_else(|_| {
            Err(io::Error::other(format!(
                "{which} forwarding thread panicked"
            )))
        })
    }

    let out = collect(out_handle.join(), "stdout");
    let err = collect(err_handle.join(), "stderr");
    Ok((out?, err?))
}

/// Run `command` with both output streams piped, mirroring each to fledge's
/// **stderr** as it arrives while capturing it for the caller.
///
/// Mirroring deliberately targets stderr, never stdout: in `--json` mode
/// stdout must stay a single parseable envelope, so child bytes may not be
/// interleaved with it. The envelope still reports `stdout` and `stderr`
/// separately and in full.
///
/// stdin is inherited (`spawn`'s default), so a streamed task can prompt —
/// unlike the buffered `Command::output` path, which closes the child's stdin.
///
/// Ordering guarantee: each stream is forwarded in order, and chunks are
/// written under a lock so a chunk is never split by the other stream. The
/// *relative* interleaving of stdout and stderr is best-effort — they are two
/// OS pipes drained by two threads, so exact cross-stream ordering cannot be
/// reconstructed. Only the inherited-terminal path (human-readable mode)
/// preserves true interleaving, because there the child writes to the
/// terminal itself.
///
/// A failure to mirror (fledge's stderr is a closed pipe, a full disk, ...)
/// does not fail the run: it is reported in `StreamedOutput::mirror_error`
/// while the status and the full capture are still returned.
fn run_streaming(command: &mut Command) -> io::Result<StreamedOutput> {
    // `io::stderr()` (not a held `.lock()`) on purpose: each `write_all` takes
    // the lock for the duration of that one chunk and releases it, so the two
    // threads interleave at chunk granularity instead of one starving the
    // other until its stream closes.
    stream_child(command, io::stderr)
}

/// The body of [`run_streaming`], generic over where the mirrored bytes go so
/// tests can point it at a sink that fails.
///
/// `make_sink` is called once per stream — each forwarding thread owns its own
/// handle (for the real thing, two `io::Stderr` handles onto the same
/// per-write-locked stream).
fn stream_child<W: Write + Send + 'static>(
    command: &mut Command,
    mut make_sink: impl FnMut() -> W,
) -> io::Result<StreamedOutput> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("child stdout pipe missing"))?;
    let child_stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("child stderr pipe missing"))?;

    let mut out_sink = make_sink();
    let mut err_sink = make_sink();
    let out_handle = std::thread::spawn(move || pump(child_stdout, &mut out_sink));
    let err_handle = std::thread::spawn(move || pump(child_stderr, &mut err_sink));

    let status = match child.wait() {
        Ok(status) => status,
        Err(e) => {
            // Same rule as `join_pumps`: never leave a forwarding thread
            // detached and still writing. Killing the child closes its pipes,
            // so both pumps reach EOF and the joins below terminate.
            let _ = child.kill();
            let _ = join_pumps(out_handle, err_handle);
            return Err(e);
        }
    };
    let (out, err) = join_pumps(out_handle, err_handle)?;

    Ok(StreamedOutput {
        status,
        stdout: out.captured,
        stderr: err.captured,
        // Report at most one; the cause is almost always shared (the same
        // broken stderr), and one warning line is enough.
        mirror_error: out.mirror_error.or(err.mirror_error),
    })
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
    let envelope = crate::envelope::action(
        RUN_LIST_SCHEMA,
        "run_list",
        serde_json::json!({
            "auto_detected": auto_detected,
            "tasks": task_list,
        }),
    );
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

fn execute_task(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    project_dir: &Path,
    visited: &mut HashSet<String>,
    json: bool,
    stream: bool,
    args: &[String],
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

    // Pass-through args apply to the named task only — dependencies run clean.
    // `stream` is an output mode, not a task input, so it does propagate.
    for dep in task.deps() {
        execute_task(dep, tasks, project_dir, visited, json, stream, &[])?;
    }

    let cmd_str = task.cmd();
    let work_dir = match task.dir() {
        Some(d) => project_dir.join(d),
        None => project_dir.to_path_buf(),
    };

    let mut command = build_task_command(cmd_str, &work_dir, task.env(), args);

    if json {
        // Both paths produce identical envelope fields. `--stream` only
        // changes *when* the bytes become visible (live, mirrored to stderr)
        // and whether the child keeps stdin.
        let (status, out_bytes, err_bytes) = if stream {
            let streamed =
                run_streaming(&mut command).with_context(|| format!("running task '{name}'"))?;
            if let Some(e) = &streamed.mirror_error {
                // Best-effort: the sink that just failed is the one we would
                // warn on, so ignore a failure to warn. The envelope below is
                // the authoritative record either way.
                let _ = writeln!(
                    io::stderr(),
                    "warning: live output for task '{name}' stopped ({e}); \
                     the task still ran — its full output is in the JSON envelope"
                );
            }
            (streamed.status, streamed.stdout, streamed.stderr)
        } else {
            let output = command
                .output()
                .with_context(|| format!("running task '{name}'"))?;
            (output.status, output.stdout, output.stderr)
        };

        let mut result = crate::envelope::action(
            RUN_TASK_SCHEMA,
            "run_task",
            serde_json::json!({
                "task": name,
                "command": cmd_str,
                "exit_code": status.code().unwrap_or(-1),
                "success": status.success(),
                "stdout": String::from_utf8_lossy(&out_bytes),
                "stderr": String::from_utf8_lossy(&err_bytes),
            }),
        );
        // Only surface `args` when present, so arg-less runs keep their exact
        // existing envelope shape.
        if !args.is_empty() {
            result["args"] = serde_json::json!(args);
        }
        println!("{}", serde_json::to_string_pretty(&result)?);

        if !status.success() {
            let code = status.code().unwrap_or(1);
            bail!("Task '{}' failed with exit code {}", name, code);
        }
    } else {
        // Human-readable mode inherits fledge's stdio, so child output is
        // already live and interleaved exactly as the child wrote it —
        // `--stream` asks for a guarantee this path always had.
        println!(
            "{} {}",
            style("▶️").cyan().bold(),
            style(format!("Running task: {name}")).bold()
        );

        let started = std::time::Instant::now();
        let status = command
            .status()
            .with_context(|| format!("running task '{name}'"))?;
        let elapsed = started.elapsed();

        if !status.success() {
            let code = status.code().unwrap_or(1);
            bail!("Task '{}' failed with exit code {}", name, code);
        }

        println!(
            "{} {} ({:.1}s)",
            style("✅").green().bold(),
            style(name).bold(),
            elapsed.as_secs_f64()
        );
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
        _ => r##"# build = "make build"
# test = "make test"
# lint = "echo 'add your linter'""##
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
        let envelope = crate::envelope::action(
            RUN_INIT_SCHEMA,
            "run_init",
            serde_json::json!({
                "file": "fledge.toml",
                "project_type": project_type,
                "files_created": ["fledge.toml"],
            }),
        );
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
    fn references_positional_detects_shell_positionals() {
        for cmd in [
            "echo $1",
            "deploy $@",
            "spread $*",
            "braced ${1}",
            "braced ${@}",
            "later $9 args",
            "mid $2 of cmd",
        ] {
            assert!(references_positional(cmd), "expected positional in: {cmd}");
        }
    }

    #[test]
    fn references_positional_ignores_non_positionals() {
        for cmd in [
            "cargo test",
            "echo hi",
            "npm run build",
            "echo $HOME",
            "echo $FOO_BAR",
            "make $$", // PID, not a positional
            "trailing dollar $",
        ] {
            assert!(
                !references_positional(cmd),
                "did not expect positional in: {cmd}"
            );
        }
    }

    #[cfg(unix)]
    fn run_cmd(cmd: &str, args: &[&str]) -> String {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let env = BTreeMap::new();
        let out = build_task_command(cmd, &std::env::temp_dir(), &env, &owned)
            .output()
            .expect("spawn task command");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_appends_args_when_no_positional() {
        // `echo` has no positional ref → fledge appends `"$@"`.
        assert_eq!(
            run_cmd("echo", &["hello", "world"]).trim_end(),
            "hello world"
        );
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_forwards_a_value() {
        // The "version bump" use case: `fledge run set-version -- 1.2.3`.
        assert_eq!(run_cmd("echo", &["1.2.3"]).trim_end(), "1.2.3");
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_no_args_is_unchanged() {
        // Backward-compat: with no pass-through args the command runs verbatim.
        assert_eq!(run_cmd("echo hi", &[]).trim_end(), "hi");
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_respects_explicit_positional_without_doubling() {
        // Command references $1 itself → args fill positionals, no auto-append,
        // so the second arg is NOT echoed a second time.
        let out = run_cmd("printf 'first=%s\\n' \"$1\"", &["A", "B"]);
        assert_eq!(out, "first=A\n");
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_does_not_allow_command_injection() {
        // A value that would be catastrophic if spliced into the shell string.
        // It must be passed as one inert literal argument instead.
        let tmp = tempfile::tempdir().unwrap();
        let env = BTreeMap::new();
        let payload = "; touch PWNED".to_string();
        let out = build_task_command("echo", tmp.path(), &env, std::slice::from_ref(&payload))
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("; touch PWNED"),
            "literal not echoed: {stdout}"
        );
        assert!(
            !tmp.path().join("PWNED").exists(),
            "injection executed — PWNED file was created"
        );
    }

    /// A sink that accepts `budget` bytes and then refuses everything — a
    /// downstream consumer that exited, or a disk that filled up.
    struct FailAfter {
        budget: usize,
        written: Vec<u8>,
    }

    impl Write for FailAfter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.budget == 0 {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "sink closed"));
            }
            let n = buf.len().min(self.budget);
            self.written.extend_from_slice(&buf[..n]);
            self.budget -= n;
            Ok(n)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A child pipe that dies mid-read — a genuine error, unlike a mirror
    /// failure.
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "pipe died"))
        }
    }

    #[test]
    fn pump_captures_and_mirrors_the_same_bytes() {
        let mut sink: Vec<u8> = Vec::new();
        let out = pump(std::io::Cursor::new(b"progress...\nmore\n"), &mut sink).unwrap();
        assert_eq!(out.captured, b"progress...\nmore\n");
        assert_eq!(
            sink, out.captured,
            "mirror must be byte-identical to capture"
        );
        assert!(out.mirror_error.is_none());
    }

    #[test]
    fn pump_forwards_partial_lines_verbatim() {
        // A prompt with no trailing newline must still reach the sink — this
        // is what makes an interactive streamed task usable.
        let mut sink: Vec<u8> = Vec::new();
        let out = pump(std::io::Cursor::new(b"Continue? [y/N] "), &mut sink).unwrap();
        assert_eq!(sink, b"Continue? [y/N] ");
        assert_eq!(out.captured, b"Continue? [y/N] ");
    }

    #[test]
    fn pump_handles_empty_input() {
        let mut sink: Vec<u8> = Vec::new();
        let out = pump(std::io::Cursor::new(b""), &mut sink).unwrap();
        assert!(out.captured.is_empty());
        assert!(sink.is_empty());
        assert!(out.mirror_error.is_none());
    }

    #[test]
    fn pump_handles_payloads_larger_than_the_buffer() {
        let big: Vec<u8> = std::iter::repeat_n(b'x', 8192 * 3 + 17).collect();
        let mut sink: Vec<u8> = Vec::new();
        let out = pump(std::io::Cursor::new(big.clone()), &mut sink).unwrap();
        assert_eq!(out.captured, big);
        assert_eq!(sink, big);
    }

    /// Regression: a write failure on the *mirror* must not discard the
    /// capture. Losing the echo may not lose the result.
    #[test]
    fn pump_keeps_capturing_after_the_mirror_stops_accepting_writes() {
        let payload: Vec<u8> = std::iter::repeat_n(b'y', 8192 * 2 + 5).collect();
        let mut sink = FailAfter {
            budget: 100,
            written: Vec::new(),
        };
        let out = pump(std::io::Cursor::new(payload.clone()), &mut sink).unwrap();
        assert_eq!(
            out.captured, payload,
            "capture must be complete even though mirroring broke"
        );
        assert_eq!(
            out.mirror_error.map(|e| e.kind()),
            Some(io::ErrorKind::BrokenPipe),
            "the mirror failure must be reported, not swallowed"
        );
        assert_eq!(
            sink.written.len(),
            100,
            "mirroring must stop at the first failure rather than retry per chunk"
        );
    }

    /// The other half of the asymmetry: a failure to *read* the child's pipe
    /// is still a real error, because the capture would be incomplete.
    #[test]
    fn pump_propagates_read_failures() {
        let mut sink: Vec<u8> = Vec::new();
        let err = pump(FailingReader, &mut sink).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    /// Regression: a broken mirror must still yield the child's true exit
    /// status and full output, so `execute_task` can build the envelope.
    #[cfg(unix)]
    #[test]
    fn stream_child_survives_a_broken_mirror_and_reports_the_real_status() {
        let env = BTreeMap::new();
        let mut cmd = build_task_command(
            "echo before; echo oops 1>&2; exit 7",
            &std::env::temp_dir(),
            &env,
            &[],
        );
        let streamed = stream_child(&mut cmd, || FailAfter {
            budget: 0,
            written: Vec::new(),
        })
        .expect("a mirror failure must not fail the run");
        assert_eq!(streamed.status.code(), Some(7));
        assert_eq!(String::from_utf8_lossy(&streamed.stdout), "before\n");
        assert_eq!(String::from_utf8_lossy(&streamed.stderr), "oops\n");
        assert!(
            streamed.mirror_error.is_some(),
            "the caller must be able to warn about the lost live output"
        );
    }

    /// Regression: a failing stdout pump must not leave the stderr forwarding
    /// thread detached and still writing while the CLI unwinds.
    #[test]
    fn join_pumps_joins_both_threads_before_propagating_an_error() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let finished = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&finished);

        let out_handle: PumpHandle =
            std::thread::spawn(|| Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdout died")));
        let err_handle: PumpHandle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            flag.store(true, Ordering::SeqCst);
            Ok(PumpOutcome {
                captured: Vec::new(),
                mirror_error: None,
            })
        });

        let err = join_pumps(out_handle, err_handle).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
        assert!(
            finished.load(Ordering::SeqCst),
            "the stderr thread must have been joined, not detached"
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_streaming_captures_both_streams_separately() {
        let env = BTreeMap::new();
        let mut cmd = build_task_command(
            "printf 'out1\\nout2\\n'; printf 'err1\\n' 1>&2",
            &std::env::temp_dir(),
            &env,
            &[],
        );
        let streamed = run_streaming(&mut cmd).unwrap();
        assert!(streamed.status.success());
        // Per-stream ordering is guaranteed; cross-stream interleaving is not,
        // so only per-stream content is asserted.
        assert_eq!(String::from_utf8_lossy(&streamed.stdout), "out1\nout2\n");
        assert_eq!(String::from_utf8_lossy(&streamed.stderr), "err1\n");
    }

    #[cfg(unix)]
    #[test]
    fn run_streaming_propagates_exit_code() {
        let env = BTreeMap::new();
        let mut cmd = build_task_command("echo before; exit 7", &std::env::temp_dir(), &env, &[]);
        let streamed = run_streaming(&mut cmd).unwrap();
        assert!(!streamed.status.success());
        assert_eq!(streamed.status.code(), Some(7));
        // Output produced before the failure is still captured.
        assert_eq!(String::from_utf8_lossy(&streamed.stdout), "before\n");
    }

    #[cfg(unix)]
    #[test]
    fn run_streaming_matches_buffered_capture() {
        // The envelope must not depend on which execution path produced it.
        let env = BTreeMap::new();
        let script = "printf 'a\\nb\\n'; printf 'z\\n' 1>&2";
        let buffered = build_task_command(script, &std::env::temp_dir(), &env, &[])
            .output()
            .unwrap();
        let mut cmd = build_task_command(script, &std::env::temp_dir(), &env, &[]);
        let streamed = run_streaming(&mut cmd).unwrap();
        assert_eq!(streamed.stdout, buffered.stdout);
        assert_eq!(streamed.stderr, buffered.stderr);
        assert_eq!(streamed.status.code(), buffered.status.code());
    }

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
        let result = execute_task(
            "a",
            &config.tasks,
            &project_dir,
            &mut visited,
            false,
            false,
            &[],
        );
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
    fn task_defaults_are_valid_toml_when_uncommented() {
        // Commented-out example tasks (`# lint = "..."`) must stay valid TOML
        // once the user uncomments them.
        let dir = tempfile::tempdir().unwrap();
        for project_type in &["python", "swift", "generic"] {
            let defaults = task_defaults(project_type, dir.path());
            let uncommented: String = defaults
                .lines()
                .map(|line| {
                    let line = line.strip_prefix("# ").unwrap_or(line);
                    format!("{line}\n")
                })
                .collect();
            let toml_str = format!("[tasks]\n{}", uncommented);
            let result: Result<FledgeFile, _> = toml::from_str(&toml_str);
            assert!(
                result.is_ok(),
                "Invalid TOML for {} after uncommenting: {:?}\n{}",
                project_type,
                result.err(),
                toml_str
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
