use anyhow::{bail, Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct PluginContext {
    #[serde(rename = "type")]
    msg_type: &'static str,
    protocol: &'static str,
    args: Vec<String>,
    project: Option<ProjectContext>,
    plugin: PluginInfo,
    fledge: FledgeInfo,
    capabilities: CapabilitiesInfo,
}

#[derive(Debug, Serialize)]
struct CapabilitiesInfo {
    exec: bool,
    store: bool,
    metadata: bool,
}

#[derive(Debug, Serialize)]
struct ProjectContext {
    name: String,
    root: String,
    language: String,
    git: Option<GitContext>,
}

#[derive(Debug, Serialize)]
struct GitContext {
    branch: String,
    dirty: bool,
    remote: String,
    remote_url: String,
}

#[derive(Debug, Serialize)]
struct PluginInfo {
    name: String,
    version: String,
    dir: String,
}

#[derive(Debug, Serialize)]
struct FledgeInfo {
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum OutboundMessage {
    Prompt {
        id: String,
        message: String,
        #[serde(default)]
        default: Option<String>,
        #[serde(default)]
        validate: Option<String>,
    },
    Confirm {
        id: String,
        message: String,
        #[serde(default)]
        default: Option<bool>,
    },
    Select {
        id: String,
        message: String,
        options: Vec<String>,
        #[serde(default)]
        default: Option<usize>,
    },
    MultiSelect {
        id: String,
        message: String,
        options: Vec<String>,
        #[serde(default)]
        defaults: Option<Vec<usize>>,
    },
    Progress {
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        current: Option<u64>,
        #[serde(default)]
        total: Option<u64>,
        #[serde(default)]
        done: Option<bool>,
    },
    Log {
        level: String,
        message: String,
    },
    Output {
        text: String,
    },
    Store {
        key: String,
        value: String,
    },
    Load {
        id: String,
        key: String,
    },
    Exec {
        id: String,
        command: String,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        timeout: Option<u64>,
    },
    Metadata {
        id: String,
        keys: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
struct InboundResponse {
    #[serde(rename = "type")]
    msg_type: &'static str,
    id: String,
    value: serde_json::Value,
}

pub fn run_protocol_plugin(
    bin_path: &Path,
    args: &[String],
    plugin_name: &str,
    plugin_version: &str,
    plugin_dir: &Path,
    capabilities: &crate::plugin::PluginCapabilities,
) -> Result<()> {
    let mut child = Command::new(bin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawning plugin '{plugin_name}'"))?;

    let project_ctx = detect_project_context();
    let init_msg = PluginContext {
        msg_type: "init",
        protocol: "fledge-v1",
        args: args.to_vec(),
        project: project_ctx,
        plugin: PluginInfo {
            name: plugin_name.to_string(),
            version: plugin_version.to_string(),
            dir: plugin_dir.to_string_lossy().to_string(),
        },
        fledge: FledgeInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        capabilities: CapabilitiesInfo {
            exec: capabilities.exec,
            store: capabilities.store,
            metadata: capabilities.metadata,
        },
    };

    send_message(&mut child, &init_msg)?;

    let result = run_message_loop(&mut child, plugin_name, plugin_dir, capabilities);

    let status = child.wait().context("waiting for plugin to exit")?;

    result?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Plugin '{}' exited with code {}", plugin_name, code);
    }

    Ok(())
}

fn run_message_loop(
    child: &mut Child,
    plugin_name: &str,
    plugin_dir: &Path,
    capabilities: &crate::plugin::PluginCapabilities,
) -> Result<()> {
    let stdout = child
        .stdout
        .take()
        .context("failed to capture plugin stdout")?;
    let reader = BufReader::new(stdout);

    let mut progress_bar: Option<ProgressBar> = None;

    for line in reader.lines() {
        let line = line.context("reading plugin output")?;
        if line.trim().is_empty() {
            continue;
        }

        let msg: OutboundMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => {
                eprintln!(
                    "  {} {}: malformed JSON, skipping",
                    style("⚠").yellow(),
                    style(plugin_name).dim()
                );
                continue;
            }
        };

        match msg {
            OutboundMessage::Prompt {
                id,
                message,
                default,
                validate,
            } => {
                clear_progress(&mut progress_bar);
                let value = handle_prompt(&message, default.as_deref(), validate.as_deref())?;
                send_response(child, &id, serde_json::Value::String(value))?;
            }
            OutboundMessage::Confirm {
                id,
                message,
                default,
            } => {
                clear_progress(&mut progress_bar);
                let value = handle_confirm(&message, default.unwrap_or(false))?;
                send_response(child, &id, serde_json::Value::Bool(value))?;
            }
            OutboundMessage::Select {
                id,
                message,
                options,
                default,
            } => {
                clear_progress(&mut progress_bar);
                let value = handle_select(&message, &options, default)?;
                send_response(child, &id, serde_json::Value::String(value))?;
            }
            OutboundMessage::MultiSelect {
                id,
                message,
                options,
                defaults,
            } => {
                clear_progress(&mut progress_bar);
                let values = handle_multi_select(&message, &options, defaults.as_deref())?;
                let json_values: Vec<serde_json::Value> =
                    values.into_iter().map(serde_json::Value::String).collect();
                send_response(child, &id, serde_json::Value::Array(json_values))?;
            }
            OutboundMessage::Progress {
                message,
                current,
                total,
                done,
            } => {
                handle_progress(
                    &mut progress_bar,
                    plugin_name,
                    message.as_deref(),
                    current,
                    total,
                    done.unwrap_or(false),
                );
            }
            OutboundMessage::Log { level, message } => {
                clear_progress(&mut progress_bar);
                handle_log(plugin_name, &level, &message);
            }
            OutboundMessage::Output { text } => {
                clear_progress(&mut progress_bar);
                print!("{}", text);
            }
            OutboundMessage::Store { key, value } => {
                if !capabilities.store {
                    let msg = format!(
                        "  {} [{}] store blocked — capability not granted (key: {})",
                        style("WARN").yellow(),
                        style(plugin_name).dim(),
                        key,
                    );
                    eprintln!("{msg}");
                    handle_log(plugin_name, "error", "store capability not granted");
                    continue;
                }
                handle_store(plugin_dir, &key, &value)?;
            }
            OutboundMessage::Load { id, key } => {
                if !capabilities.store {
                    eprintln!(
                        "  {} [{}] load blocked — capability not granted",
                        style("WARN").yellow(),
                        style(plugin_name).dim()
                    );
                    send_response(child, &id, serde_json::Value::Null)?;
                    continue;
                }
                let value = handle_load(plugin_dir, &key)?;
                send_response(child, &id, value)?;
            }
            OutboundMessage::Exec {
                id,
                command,
                cwd,
                timeout,
            } => {
                if !capabilities.exec {
                    eprintln!(
                        "  {} [{}] exec blocked — capability not granted",
                        style("WARN").yellow(),
                        style(plugin_name).dim()
                    );
                    send_response(
                        child,
                        &id,
                        serde_json::json!({
                            "code": 126,
                            "stdout": "",
                            "stderr": "exec capability not granted"
                        }),
                    )?;
                    continue;
                }
                let result = handle_exec(&command, cwd.as_deref(), timeout, plugin_dir)?;
                send_response(child, &id, result)?;
            }
            OutboundMessage::Metadata { id, keys } => {
                if !capabilities.metadata {
                    eprintln!(
                        "  {} [{}] metadata blocked — capability not granted",
                        style("WARN").yellow(),
                        style(plugin_name).dim()
                    );
                    let empty = serde_json::Value::Object(serde_json::Map::new());
                    send_response(child, &id, empty)?;
                    continue;
                }
                let result = handle_metadata(&keys)?;
                send_response(child, &id, result)?;
            }
        }
    }

    clear_progress(&mut progress_bar);
    Ok(())
}

fn send_message<T: Serialize>(child: &mut Child, msg: &T) -> Result<()> {
    send_raw(child, msg)
}

fn send_raw<T: Serialize>(child: &mut Child, msg: &T) -> Result<()> {
    let stdin = child.stdin.as_mut().context("plugin stdin unavailable")?;
    let json = serde_json::to_string(msg).context("serializing message")?;
    writeln!(stdin, "{}", json).context("writing to plugin stdin")?;
    stdin.flush().context("flushing plugin stdin")?;
    Ok(())
}

fn send_response(child: &mut Child, id: &str, value: serde_json::Value) -> Result<()> {
    send_raw(
        child,
        &InboundResponse {
            msg_type: "response",
            id: id.to_string(),
            value,
        },
    )
}

fn handle_prompt(message: &str, default: Option<&str>, validate: Option<&str>) -> Result<String> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut prompt = dialoguer::Input::<String>::with_theme(&theme).with_prompt(message);

    if let Some(d) = default {
        prompt = prompt.default(d.to_string());
    }

    if let Some(v) = validate {
        match v {
            "non_empty" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    if input.trim().is_empty() {
                        Err("Input cannot be empty".to_string())
                    } else {
                        Ok(())
                    }
                });
            }
            "integer" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    input
                        .parse::<i64>()
                        .map(|_| ())
                        .map_err(|_| "Must be an integer".to_string())
                });
            }
            "path_exists" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    if Path::new(input).exists() {
                        Ok(())
                    } else {
                        Err("Path does not exist".to_string())
                    }
                });
            }
            "url" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    if input.starts_with("http://") || input.starts_with("https://") {
                        Ok(())
                    } else {
                        Err("Must be a valid URL (http:// or https://)".to_string())
                    }
                });
            }
            _ => {}
        }
    }

    prompt.interact_text().context("reading user input")
}

fn handle_confirm(message: &str, default: bool) -> Result<bool> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    dialoguer::Confirm::with_theme(&theme)
        .with_prompt(message)
        .default(default)
        .interact()
        .context("reading confirmation")
}

fn handle_select(message: &str, options: &[String], default: Option<usize>) -> Result<String> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut select = dialoguer::Select::with_theme(&theme)
        .with_prompt(message)
        .items(options);

    if let Some(d) = default {
        select = select.default(d);
    }

    let idx = select.interact().context("reading selection")?;
    Ok(options[idx].clone())
}

fn handle_multi_select(
    message: &str,
    options: &[String],
    defaults: Option<&[usize]>,
) -> Result<Vec<String>> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut select = dialoguer::MultiSelect::with_theme(&theme)
        .with_prompt(message)
        .items(options);

    if let Some(d) = defaults {
        let bools: Vec<bool> = (0..options.len()).map(|i| d.contains(&i)).collect();
        select = select.defaults(&bools);
    }

    let indices = select.interact().context("reading multi-selection")?;
    Ok(indices.into_iter().map(|i| options[i].clone()).collect())
}

fn handle_progress(
    bar: &mut Option<ProgressBar>,
    plugin_name: &str,
    message: Option<&str>,
    current: Option<u64>,
    total: Option<u64>,
    done: bool,
) {
    if done {
        clear_progress(bar);
        return;
    }

    let msg = message.unwrap_or("Working");

    match (current, total) {
        (Some(cur), Some(tot)) => {
            let pb = bar.get_or_insert_with(|| {
                let pb = ProgressBar::new(tot);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(&format!(
                            "  {} {{msg}} [{{bar:30}}] {{pos}}/{{len}}",
                            style("▶").cyan().bold()
                        ))
                        .expect("valid bar template")
                        .progress_chars("==>  "),
                );
                pb
            });
            pb.set_length(tot);
            pb.set_position(cur);
            pb.set_message(format!("{} ({})", msg, plugin_name));
        }
        _ => {
            if bar.is_none() {
                let sp = ProgressBar::new_spinner();
                sp.set_style(
                    ProgressStyle::default_spinner()
                        .template(&format!(
                            "  {} {{msg}} {{spinner}}",
                            style("▶").cyan().bold()
                        ))
                        .expect("valid spinner template"),
                );
                sp.enable_steady_tick(Duration::from_millis(100));
                *bar = Some(sp);
            }
            if let Some(pb) = bar.as_ref() {
                pb.set_message(format!("{} ({})", msg, plugin_name));
            }
        }
    }
}

fn clear_progress(bar: &mut Option<ProgressBar>) {
    if let Some(pb) = bar.take() {
        pb.finish_and_clear();
    }
}

fn handle_log(plugin_name: &str, level: &str, message: &str) {
    let prefix = match level {
        "debug" => style("DEBUG").dim(),
        "info" => style("INFO").cyan(),
        "warn" => style("WARN").yellow(),
        "error" => style("ERROR").red().bold(),
        _ => style(level).dim(),
    };
    eprintln!("  {} [{}] {}", prefix, style(plugin_name).dim(), message);
}

const MAX_STORE_KEY_SIZE: usize = 256;
const MAX_STORE_VALUE_SIZE: usize = 64 * 1024; // 64 KB per value
const MAX_STORE_TOTAL_SIZE: usize = 1024 * 1024; // 1 MB total
const MAX_STORE_KEY_COUNT: usize = 256;

fn handle_store(plugin_dir: &Path, key: &str, value: &str) -> Result<()> {
    if key.is_empty() {
        bail!("store key must not be empty");
    }
    if key.len() > MAX_STORE_KEY_SIZE {
        bail!(
            "store key exceeds maximum size of {} bytes",
            MAX_STORE_KEY_SIZE
        );
    }
    if key.bytes().any(|b| b < 0x20 || b == 0x7f) {
        bail!("store key contains control characters");
    }
    if value.len() > MAX_STORE_VALUE_SIZE {
        bail!(
            "store value exceeds maximum size of {} bytes",
            MAX_STORE_VALUE_SIZE
        );
    }

    fs::create_dir_all(plugin_dir).context("creating plugin directory")?;
    let state_path = plugin_dir.join("state.json");
    let lock_path = plugin_dir.join("state.json.lock");

    let lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .context("opening state lock file")?;
    lock_file
        .lock()
        .context("acquiring exclusive lock on state.json")?;

    let mut state: HashMap<String, String> = if state_path.exists() {
        let content = fs::read_to_string(&state_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };
    if !state.contains_key(key) && state.len() >= MAX_STORE_KEY_COUNT {
        bail!(
            "plugin state exceeds maximum of {} keys",
            MAX_STORE_KEY_COUNT
        );
    }
    state.insert(key.to_string(), value.to_string());
    let json = serde_json::to_string_pretty(&state).context("serializing state")?;
    if json.len() > MAX_STORE_TOTAL_SIZE {
        bail!(
            "plugin state exceeds maximum total size of {} bytes",
            MAX_STORE_TOTAL_SIZE
        );
    }

    let tmp_path = plugin_dir.join("state.json.tmp");
    fs::write(&tmp_path, &json).context("writing temporary state file")?;
    #[cfg(windows)]
    {
        let _ = fs::remove_file(&state_path);
    }
    fs::rename(&tmp_path, &state_path).context("replacing state.json")?;

    lock_file.unlock().context("releasing state.json lock")?;
    Ok(())
}

fn handle_load(plugin_dir: &Path, key: &str) -> Result<serde_json::Value> {
    let state_path = plugin_dir.join("state.json");
    let lock_path = plugin_dir.join("state.json.lock");

    let lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .context("opening state lock file")?;
    lock_file
        .lock_shared()
        .context("acquiring shared lock on state.json")?;

    if !state_path.exists() {
        lock_file.unlock().context("releasing state.json lock")?;
        return Ok(serde_json::Value::Null);
    }

    let content = fs::read_to_string(&state_path).context("reading state.json")?;

    lock_file.unlock().context("releasing state.json lock")?;

    let state: HashMap<String, String> = serde_json::from_str(&content).unwrap_or_default();
    match state.get(key) {
        Some(v) => Ok(serde_json::Value::String(v.clone())),
        None => Ok(serde_json::Value::Null),
    }
}

fn handle_exec(
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

fn handle_metadata(keys: &[String]) -> Result<serde_json::Value> {
    let mut result = serde_json::Map::new();

    for key in keys {
        match key.as_str() {
            "fledge_config" => {
                let config_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("fledge.toml");
                if config_path.exists() {
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        if let Ok(parsed) = content.parse::<toml::Value>() {
                            result.insert(
                                key.clone(),
                                serde_json::to_value(parsed).unwrap_or(serde_json::Value::Null),
                            );
                            continue;
                        }
                    }
                }
                result.insert(key.clone(), serde_json::Value::Null);
            }
            "git_tags" => {
                let tags: Vec<String> = Command::new("git")
                    .args(["tag", "--sort=-v:refname"])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key.clone(),
                    serde_json::to_value(tags).unwrap_or(serde_json::Value::Null),
                );
            }
            "git_status" => {
                let files: Vec<String> = Command::new("git")
                    .args(["status", "--porcelain"])
                    .output()
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key.clone(),
                    serde_json::to_value(files).unwrap_or(serde_json::Value::Null),
                );
            }
            "git_log" => {
                let entries: Vec<String> = Command::new("git")
                    .args(["log", "--oneline", "-20"])
                    .output()
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key.clone(),
                    serde_json::to_value(entries).unwrap_or(serde_json::Value::Null),
                );
            }
            "env" => {
                let sensitive_patterns = [
                    "secret",
                    "token",
                    "password",
                    "key",
                    "credential",
                    "auth",
                    "private",
                    "session",
                    "cookie",
                ];
                let dangerous_prefixes = ["ld_preload", "ld_library_path", "dyld_", "kubeconfig"];
                let safe_vars: HashMap<String, String> = std::env::vars()
                    .filter(|(k, v)| {
                        let lower = k.to_lowercase();
                        let is_sensitive_name =
                            sensitive_patterns.iter().any(|p| lower.contains(p));
                        let is_dangerous_prefix =
                            dangerous_prefixes.iter().any(|p| lower.starts_with(p));
                        let looks_like_conn_string = lower.ends_with("_url")
                            || lower.ends_with("_uri")
                            || lower.ends_with("_dsn");
                        let value_has_creds = v.contains('@') && v.contains(':');
                        !is_sensitive_name
                            && !is_dangerous_prefix
                            && !looks_like_conn_string
                            && !value_has_creds
                    })
                    .collect();
                result.insert(
                    key.clone(),
                    serde_json::to_value(safe_vars).unwrap_or(serde_json::Value::Null),
                );
            }
            _ => {
                result.insert(key.clone(), serde_json::Value::Null);
            }
        }
    }

    Ok(serde_json::Value::Object(result))
}

fn detect_project_context() -> Option<ProjectContext> {
    let root = std::env::current_dir().ok()?;

    let language = crate::run::detect_project_type(&root).to_string();

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git = detect_git_context(&root);

    Some(ProjectContext {
        name,
        root: root.to_string_lossy().to_string(),
        language,
        git,
    })
}

fn sanitize_remote_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some(at_pos) = rest.find('@') {
            return format!("https://{}", &rest[at_pos + 1..]);
        }
    } else if let Some(rest) = url.strip_prefix("http://") {
        if let Some(at_pos) = rest.find('@') {
            return format!("http://{}", &rest[at_pos + 1..]);
        }
    }
    url.to_string()
}

fn detect_git_context(root: &Path) -> Option<GitContext> {
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let remote = Command::new("git")
        .args(["remote"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("origin")
                .to_string()
        })
        .unwrap_or_else(|| "origin".to_string());

    let remote_url = Command::new("git")
        .args(["remote", "get-url", &remote])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| sanitize_remote_url(String::from_utf8_lossy(&o.stdout).trim()))
        .unwrap_or_default();

    Some(GitContext {
        branch,
        dirty,
        remote,
        remote_url,
    })
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<std::process::Output, ()> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    std::io::Read::read_to_end(&mut out, &mut stdout).ok();
                }
                if let Some(mut err) = child.stderr.take() {
                    std::io::Read::read_to_end(&mut err, &mut stderr).ok();
                }
                let status = child
                    .wait()
                    .unwrap_or_else(|_| std::process::Command::new("true").status().unwrap());
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

fn kill_child(child: &mut Child) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn all_capabilities() -> crate::plugin::PluginCapabilities {
        crate::plugin::PluginCapabilities {
            exec: true,
            store: true,
            metadata: true,
        }
    }

    fn no_capabilities() -> crate::plugin::PluginCapabilities {
        crate::plugin::PluginCapabilities {
            exec: false,
            store: false,
            metadata: false,
        }
    }

    #[test]
    fn parse_prompt_message() {
        let json = r#"{"type":"prompt","id":"1","message":"Deploy target:","default":"staging"}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Prompt {
                id,
                message,
                default,
                validate,
            } => {
                assert_eq!(id, "1");
                assert_eq!(message, "Deploy target:");
                assert_eq!(default, Some("staging".to_string()));
                assert!(validate.is_none());
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_confirm_message() {
        let json = r#"{"type":"confirm","id":"2","message":"Deploy?","default":false}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Confirm {
                id,
                message,
                default,
            } => {
                assert_eq!(id, "2");
                assert_eq!(message, "Deploy?");
                assert_eq!(default, Some(false));
            }
            _ => panic!("expected Confirm"),
        }
    }

    #[test]
    fn parse_select_message() {
        let json =
            r#"{"type":"select","id":"3","message":"Choose:","options":["a","b","c"],"default":1}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Select {
                id,
                message,
                options,
                default,
            } => {
                assert_eq!(id, "3");
                assert_eq!(message, "Choose:");
                assert_eq!(options, vec!["a", "b", "c"]);
                assert_eq!(default, Some(1));
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn parse_multi_select_message() {
        let json = r#"{"type":"multi_select","id":"4","message":"Pick:","options":["x","y"],"defaults":[0]}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::MultiSelect {
                id,
                message,
                options,
                defaults,
            } => {
                assert_eq!(id, "4");
                assert_eq!(message, "Pick:");
                assert_eq!(options, vec!["x", "y"]);
                assert_eq!(defaults, Some(vec![0]));
            }
            _ => panic!("expected MultiSelect"),
        }
    }

    #[test]
    fn parse_progress_message() {
        let json = r#"{"type":"progress","message":"Uploading","current":3,"total":10}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Progress {
                message,
                current,
                total,
                done,
            } => {
                assert_eq!(message, Some("Uploading".to_string()));
                assert_eq!(current, Some(3));
                assert_eq!(total, Some(10));
                assert_eq!(done, None);
            }
            _ => panic!("expected Progress"),
        }
    }

    #[test]
    fn parse_progress_done() {
        let json = r#"{"type":"progress","done":true}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Progress { done, .. } => {
                assert_eq!(done, Some(true));
            }
            _ => panic!("expected Progress"),
        }
    }

    #[test]
    fn parse_log_message() {
        let json = r#"{"type":"log","level":"warn","message":"No config found"}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Log { level, message } => {
                assert_eq!(level, "warn");
                assert_eq!(message, "No config found");
            }
            _ => panic!("expected Log"),
        }
    }

    #[test]
    fn parse_output_message() {
        let json = r#"{"type":"output","text":"Deployed in 4.2s\n"}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Output { text } => {
                assert_eq!(text, "Deployed in 4.2s\n");
            }
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn parse_store_message() {
        let json = r#"{"type":"store","key":"last_target","value":"prod"}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Store { key, value } => {
                assert_eq!(key, "last_target");
                assert_eq!(value, "prod");
            }
            _ => panic!("expected Store"),
        }
    }

    #[test]
    fn parse_load_message() {
        let json = r#"{"type":"load","id":"5","key":"last_target"}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Load { id, key } => {
                assert_eq!(id, "5");
                assert_eq!(key, "last_target");
            }
            _ => panic!("expected Load"),
        }
    }

    #[test]
    fn parse_exec_message() {
        let json = r#"{"type":"exec","id":"6","command":"git tag -l","timeout":10}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Exec {
                id,
                command,
                cwd,
                timeout,
            } => {
                assert_eq!(id, "6");
                assert_eq!(command, "git tag -l");
                assert!(cwd.is_none());
                assert_eq!(timeout, Some(10));
            }
            _ => panic!("expected Exec"),
        }
    }

    #[test]
    fn parse_metadata_message() {
        let json = r#"{"type":"metadata","id":"7","keys":["git_tags","git_status"]}"#;
        let msg: OutboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            OutboundMessage::Metadata { id, keys } => {
                assert_eq!(id, "7");
                assert_eq!(keys, vec!["git_tags", "git_status"]);
            }
            _ => panic!("expected Metadata"),
        }
    }

    #[test]
    fn malformed_json_is_rejected() {
        let json = r#"this is not json"#;
        let result: Result<OutboundMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_type_is_rejected() {
        let json = r#"{"type":"unknown_future_type","id":"99"}"#;
        let result: Result<OutboundMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn store_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        handle_store(tmp.path(), "test_key", "test_value").unwrap();
        let value = handle_load(tmp.path(), "test_key").unwrap();
        assert_eq!(value, serde_json::Value::String("test_value".to_string()));
    }

    #[test]
    fn load_missing_key_returns_null() {
        let tmp = tempfile::tempdir().unwrap();
        let value = handle_load(tmp.path(), "nonexistent").unwrap();
        assert_eq!(value, serde_json::Value::Null);
    }

    #[test]
    fn load_missing_state_file_returns_null() {
        let tmp = tempfile::tempdir().unwrap();
        let value = handle_load(tmp.path(), "anything").unwrap();
        assert_eq!(value, serde_json::Value::Null);
    }

    #[test]
    fn store_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        handle_store(tmp.path(), "key", "first").unwrap();
        handle_store(tmp.path(), "key", "second").unwrap();
        let value = handle_load(tmp.path(), "key").unwrap();
        assert_eq!(value, serde_json::Value::String("second".to_string()));
    }

    #[test]
    fn store_multiple_keys() {
        let tmp = tempfile::tempdir().unwrap();
        handle_store(tmp.path(), "a", "1").unwrap();
        handle_store(tmp.path(), "b", "2").unwrap();
        assert_eq!(
            handle_load(tmp.path(), "a").unwrap(),
            serde_json::Value::String("1".to_string())
        );
        assert_eq!(
            handle_load(tmp.path(), "b").unwrap(),
            serde_json::Value::String("2".to_string())
        );
    }

    #[test]
    fn init_message_serializes() {
        let ctx = PluginContext {
            msg_type: "init",
            protocol: "fledge-v1",
            args: vec!["--dry-run".to_string()],
            project: Some(ProjectContext {
                name: "test".to_string(),
                root: "/tmp/test".to_string(),
                language: "rust".to_string(),
                git: Some(GitContext {
                    branch: "main".to_string(),
                    dirty: false,
                    remote: "origin".to_string(),
                    remote_url: "https://github.com/test/test".to_string(),
                }),
            }),
            plugin: PluginInfo {
                name: "fledge-test".to_string(),
                version: "0.1.0".to_string(),
                dir: "/tmp/plugins/fledge-test".to_string(),
            },
            fledge: FledgeInfo {
                version: "0.9.1".to_string(),
            },
            capabilities: CapabilitiesInfo {
                exec: true,
                store: true,
                metadata: false,
            },
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "init");
        assert_eq!(parsed["capabilities"]["exec"], true);
        assert_eq!(parsed["capabilities"]["store"], true);
        assert_eq!(parsed["capabilities"]["metadata"], false);
        assert_eq!(parsed["protocol"], "fledge-v1");
        assert_eq!(parsed["args"][0], "--dry-run");
        assert_eq!(parsed["project"]["name"], "test");
        assert_eq!(parsed["project"]["git"]["branch"], "main");
    }

    #[test]
    fn response_serializes_correctly() {
        let resp = InboundResponse {
            msg_type: "response",
            id: "42".to_string(),
            value: serde_json::Value::String("hello".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "response");
        assert_eq!(parsed["id"], "42");
        assert_eq!(parsed["value"], "hello");
    }

    #[test]
    fn exec_sandbox_blocks_path_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let result = handle_exec("echo hi", Some("../../.."), None, tmp.path()).unwrap();
        let code = result["code"].as_i64().unwrap();
        assert_ne!(code, 0);
    }

    #[test]
    fn exec_runs_simple_command() {
        let tmp = tempfile::tempdir().unwrap();
        let result = handle_exec("echo hello", None, None, tmp.path()).unwrap();
        assert_eq!(result["code"].as_i64().unwrap(), 0);
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
    }

    #[test]
    fn metadata_handles_unknown_keys() {
        let result = handle_metadata(&["nonexistent_key".to_string()]).unwrap();
        assert_eq!(result["nonexistent_key"], serde_json::Value::Null);
    }

    fn compile_test_plugin(src: &str, tmp: &Path) -> PathBuf {
        let src_path = tmp.join("test_plugin.rs");
        std::fs::write(&src_path, src).unwrap();
        let bin_name = if cfg!(windows) {
            "test_plugin.exe"
        } else {
            "test_plugin"
        };
        let bin_path = tmp.join(bin_name);
        let output = std::process::Command::new("rustc")
            .args([src_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()])
            .output()
            .expect("rustc must be available to run plugin tests");
        assert!(
            output.status.success(),
            "rustc failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        bin_path
    }

    #[test]
    fn run_protocol_plugin_store_load() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();
    send("{\"type\":\"log\",\"level\":\"info\",\"message\":\"test started\"}");
    send("{\"type\":\"store\",\"key\":\"test_key\",\"value\":\"test_value\"}");
    send("{\"type\":\"load\",\"id\":\"load1\",\"key\":\"test_key\"}");
    let _resp = lines.next().unwrap().unwrap();
    send("{\"type\":\"output\",\"text\":\"done\\n\"}");
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-plugin",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(result.is_ok(), "protocol plugin failed: {:?}", result.err());

        let state_path = store_dir.path().join("state.json");
        assert!(state_path.exists(), "store should have created state.json");
        let state: std::collections::HashMap<String, String> =
            serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(
            state.get("test_key").map(|s| s.as_str()),
            Some("test_value")
        );
    }

    #[test]
    fn run_protocol_plugin_exec_and_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    // Test exec: run a simple cross-platform command
    send("{\"type\":\"exec\",\"id\":\"e1\",\"command\":\"echo hello_from_plugin\"}");
    let exec_resp = lines.next().unwrap().unwrap();
    assert!(exec_resp.contains("\"id\":\"e1\""), "response should echo id");
    assert!(exec_resp.contains("hello_from_plugin"), "exec stdout missing: {}", exec_resp);

    // Test metadata
    send("{\"type\":\"metadata\",\"id\":\"m1\",\"keys\":[\"env\"]}");
    let meta_resp = lines.next().unwrap().unwrap();
    assert!(meta_resp.contains("\"id\":\"m1\""), "metadata response should echo id");
    assert!(meta_resp.contains("\"env\""), "metadata should contain env key");

    send("{\"type\":\"output\",\"text\":\"exec+metadata ok\\n\"}");
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-exec",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(
            result.is_ok(),
            "exec/metadata test failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn run_protocol_plugin_graceful_exit_no_messages() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead};
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();
    // Exit immediately without sending anything
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-noop",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(
            result.is_ok(),
            "noop plugin should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn run_protocol_plugin_nonzero_exit_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead};
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();
    std::process::exit(42);
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-fail",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(result.is_err(), "nonzero exit should be an error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("42"),
            "error should mention exit code: {err_msg}"
        );
    }

    #[test]
    fn run_protocol_plugin_malformed_json_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();
    // Send garbage — should be skipped, not crash
    send("this is not json at all");
    send("{malformed");
    send("{\"type\":\"unknown_future_type\",\"id\":\"x\"}");
    // Then send valid messages
    send("{\"type\":\"store\",\"key\":\"survived\",\"value\":\"yes\"}");
    send("{\"type\":\"output\",\"text\":\"still alive\\n\"}");
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-malformed",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        // Plugin exits 0, malformed lines are skipped
        assert!(
            result.is_ok(),
            "malformed JSON should be skipped: {:?}",
            result.err()
        );
        let state: std::collections::HashMap<String, String> = serde_json::from_str(
            &std::fs::read_to_string(store_dir.path().join("state.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(state.get("survived").map(|s| s.as_str()), Some("yes"));
    }

    #[test]
    fn run_protocol_plugin_multiple_store_load_cycles() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    // Store multiple keys
    send("{\"type\":\"store\",\"key\":\"a\",\"value\":\"1\"}");
    send("{\"type\":\"store\",\"key\":\"b\",\"value\":\"2\"}");
    send("{\"type\":\"store\",\"key\":\"a\",\"value\":\"3\"}");

    // Load them back and verify via string matching
    send("{\"type\":\"load\",\"id\":\"la\",\"key\":\"a\"}");
    let resp_a = lines.next().unwrap().unwrap();
    assert!(resp_a.contains("\"id\":\"la\""), "response should echo id la");
    assert!(resp_a.contains("\"3\""), "overwritten value should be 3: {}", resp_a);

    send("{\"type\":\"load\",\"id\":\"lb\",\"key\":\"b\"}");
    let resp_b = lines.next().unwrap().unwrap();
    assert!(resp_b.contains("\"2\""), "b should be 2: {}", resp_b);

    // Load nonexistent key — should get null
    send("{\"type\":\"load\",\"id\":\"lc\",\"key\":\"nonexistent\"}");
    let resp_c = lines.next().unwrap().unwrap();
    assert!(resp_c.contains("null"), "missing key should return null: {}", resp_c);
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-multi-store",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(
            result.is_ok(),
            "multi store/load failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn run_protocol_plugin_receives_init_context() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let init_line = lines.next().unwrap().unwrap();

    // Verify init message structure via string matching (no serde_json in standalone rustc)
    assert!(init_line.contains("\"type\":\"init\""), "missing type:init");
    assert!(init_line.contains("\"protocol\":\"fledge-v1\""), "missing protocol");
    assert!(init_line.contains("\"plugin\""), "missing plugin field");
    assert!(init_line.contains("\"fledge\""), "missing fledge field");
    assert!(init_line.contains("\"name\""), "missing plugin name");
    assert!(init_line.contains("\"version\""), "missing version");

    send("{\"type\":\"log\",\"level\":\"info\",\"message\":\"init validated\"}");
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-init",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(
            result.is_ok(),
            "init context test failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn run_protocol_plugin_exec_sandbox_blocks_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    // Try to escape sandbox with nonexistent path traversal
    send("{\"type\":\"exec\",\"id\":\"e1\",\"command\":\"echo pwned\",\"cwd\":\"../../..\"}");
    let resp = lines.next().unwrap().unwrap();
    assert!(!resp.contains("\"code\":0"), "sandbox escape should be blocked: {}", resp);
    assert!(
        resp.contains("does not exist") || resp.contains("escapes project"),
        "expected sandbox error message, got: {}", resp
    );
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-sandbox",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(result.is_ok(), "sandbox test failed: {:?}", result.err());
    }

    #[test]
    fn run_protocol_plugin_exec_timeout_returns_code_124() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    // Request exec with a command that sleeps longer than the timeout
    send("{\"type\":\"exec\",\"id\":\"t1\",\"command\":\"sleep 30\",\"timeout\":1}");
    let resp = lines.next().unwrap().unwrap();
    assert!(resp.contains("\"code\":124"), "expected timeout code 124, got: {}", resp);
    assert!(resp.contains("timed out"), "expected timeout message, got: {}", resp);
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-timeout",
            "0.1.0",
            store_dir.path(),
            &all_capabilities(),
        );
        assert!(result.is_ok(), "timeout test failed: {:?}", result.err());
    }

    #[test]
    fn sanitize_remote_url_strips_credentials() {
        assert_eq!(
            super::sanitize_remote_url("https://token@github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
        assert_eq!(
            super::sanitize_remote_url("https://user:pass@github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
        assert_eq!(
            super::sanitize_remote_url("https://github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
        assert_eq!(
            super::sanitize_remote_url("git@github.com:org/repo.git"),
            "git@github.com:org/repo.git"
        );
    }

    #[test]
    fn store_rejects_empty_key() {
        let tmp = tempfile::tempdir().unwrap();
        let err = handle_store(tmp.path(), "", "value").unwrap_err();
        assert!(
            err.to_string().contains("must not be empty"),
            "expected empty key error, got: {err}"
        );
    }

    #[test]
    fn store_rejects_control_characters_in_key() {
        let tmp = tempfile::tempdir().unwrap();
        let err = handle_store(tmp.path(), "bad\x00key", "value").unwrap_err();
        assert!(
            err.to_string().contains("control characters"),
            "expected control char error, got: {err}"
        );
        let err2 = handle_store(tmp.path(), "bad\nkey", "value").unwrap_err();
        assert!(
            err2.to_string().contains("control characters"),
            "expected control char error, got: {err2}"
        );
    }

    #[test]
    fn store_creates_lock_file() {
        let tmp = tempfile::tempdir().unwrap();
        handle_store(tmp.path(), "key", "value").unwrap();
        assert!(tmp.path().join("state.json.lock").exists());
    }

    #[test]
    fn store_atomic_write() {
        let tmp = tempfile::tempdir().unwrap();
        handle_store(tmp.path(), "key", "value").unwrap();
        assert!(tmp.path().join("state.json").exists());
        assert!(!tmp.path().join("state.json.tmp").exists());
    }

    #[test]
    fn capability_blocks_exec() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    send("{\"type\":\"exec\",\"id\":\"e1\",\"command\":\"echo blocked\"}");
    let resp = lines.next().unwrap().unwrap();
    assert!(resp.contains("\"code\":126"), "exec should be blocked with code 126: {}", resp);
    assert!(resp.contains("not granted"), "should mention not granted: {}", resp);
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let caps = no_capabilities();
        let result =
            super::run_protocol_plugin(&bin, &[], "test-no-exec", "0.1.0", store_dir.path(), &caps);
        assert!(
            result.is_ok(),
            "blocked exec should not crash: {:?}",
            result.err()
        );
    }

    #[test]
    fn capability_blocks_store() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    send("{\"type\":\"store\",\"key\":\"secret\",\"value\":\"data\"}");
    send("{\"type\":\"load\",\"id\":\"l1\",\"key\":\"secret\"}");
    let resp = lines.next().unwrap().unwrap();
    assert!(resp.contains("null"), "load should return null when store blocked: {}", resp);
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let caps = no_capabilities();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-no-store",
            "0.1.0",
            store_dir.path(),
            &caps,
        );
        assert!(
            result.is_ok(),
            "blocked store should not crash: {:?}",
            result.err()
        );
        assert!(
            !store_dir.path().join("state.json").exists(),
            "state.json should not be created when store is blocked"
        );
    }

    #[test]
    fn capability_blocks_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let _init = lines.next().unwrap().unwrap();

    send("{\"type\":\"metadata\",\"id\":\"m1\",\"keys\":[\"env\"]}");
    let resp = lines.next().unwrap().unwrap();
    assert!(resp.contains("\"id\":\"m1\""), "should echo id: {}", resp);
    // Should get empty object, not env data
    assert!(!resp.contains("PATH"), "should not contain env data when blocked: {}", resp);
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let caps = no_capabilities();
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-no-metadata",
            "0.1.0",
            store_dir.path(),
            &caps,
        );
        assert!(
            result.is_ok(),
            "blocked metadata should not crash: {:?}",
            result.err()
        );
    }

    #[test]
    fn init_message_includes_capabilities() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = compile_test_plugin(
            r#"
use std::io::{self, BufRead, Write};
fn send(msg: &str) { println!("{}", msg); io::stdout().flush().unwrap(); }
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let init_line = lines.next().unwrap().unwrap();

    assert!(init_line.contains("\"capabilities\""), "init should contain capabilities: {}", init_line);
    assert!(init_line.contains("\"exec\":true"), "should have exec:true: {}", init_line);
    assert!(init_line.contains("\"store\":false"), "should have store:false: {}", init_line);
    assert!(init_line.contains("\"metadata\":true"), "should have metadata:true: {}", init_line);

    send("{\"type\":\"log\",\"level\":\"info\",\"message\":\"caps validated\"}");
}
"#,
            tmp.path(),
        );
        let store_dir = tempfile::tempdir().unwrap();
        let caps = crate::plugin::PluginCapabilities {
            exec: true,
            store: false,
            metadata: true,
        };
        let result = super::run_protocol_plugin(
            &bin,
            &[],
            "test-caps-init",
            "0.1.0",
            store_dir.path(),
            &caps,
        );
        assert!(result.is_ok(), "caps init test failed: {:?}", result.err());
    }

    #[test]
    fn capabilities_parse_from_toml() {
        let caps_str = r#"
exec = true
store = true
metadata = false
"#;
        let caps: crate::plugin::PluginCapabilities = toml::from_str(caps_str).unwrap();
        assert!(caps.exec);
        assert!(caps.store);
        assert!(!caps.metadata);
    }

    #[test]
    fn capabilities_default_to_false() {
        let caps: crate::plugin::PluginCapabilities = toml::from_str("").unwrap();
        assert!(!caps.exec);
        assert!(!caps.store);
        assert!(!caps.metadata);
    }
}
