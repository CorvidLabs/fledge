use anyhow::{bail, Context, Result};
use console::style;
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

mod detect;
mod exec;
mod metadata;
mod store;
mod ui;

#[cfg(test)]
mod tests;

// Re-export submodule items needed by tests (via `use super::*`)
#[cfg(test)]
pub(crate) use detect::sanitize_remote_url;
#[cfg(test)]
pub(crate) use exec::{handle_exec, MAX_EXEC_OUTPUT_SIZE};
#[cfg(test)]
pub(crate) use metadata::handle_metadata;
#[cfg(test)]
pub(crate) use store::{handle_load, handle_store};
// Re-export std::io::Read so tests can call .read_to_end() via `use super::*`
#[cfg(test)]
pub(crate) use std::io::Read;

#[derive(Debug, Serialize)]
pub struct PluginContext {
    #[serde(rename = "type")]
    pub(crate) msg_type: &'static str,
    pub(crate) protocol: &'static str,
    pub(crate) args: Vec<String>,
    pub(crate) project: Option<ProjectContext>,
    pub(crate) plugin: PluginInfo,
    pub(crate) fledge: FledgeInfo,
    pub(crate) capabilities: CapabilitiesInfo,
}

#[derive(Debug, Serialize)]
pub(crate) struct CapabilitiesInfo {
    pub(crate) exec: bool,
    pub(crate) store: bool,
    pub(crate) metadata: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProjectContext {
    pub(crate) name: String,
    pub(crate) root: String,
    pub(crate) language: String,
    pub(crate) git: Option<GitContext>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitContext {
    pub(crate) branch: String,
    pub(crate) dirty: bool,
    pub(crate) remote: String,
    pub(crate) remote_url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PluginInfo {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) dir: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct FledgeInfo {
    pub(crate) version: String,
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
pub(crate) struct InboundResponse {
    #[serde(rename = "type")]
    pub(crate) msg_type: &'static str,
    pub(crate) id: String,
    pub(crate) value: serde_json::Value,
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
        .env("FLEDGE_PLUGIN_DIR", plugin_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawning plugin '{plugin_name}'"))?;

    let project_ctx = detect::detect_project_context();
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
                ui::clear_progress(&mut progress_bar);
                let value = ui::handle_prompt(&message, default.as_deref(), validate.as_deref())?;
                send_response(child, &id, serde_json::Value::String(value))?;
            }
            OutboundMessage::Confirm {
                id,
                message,
                default,
            } => {
                ui::clear_progress(&mut progress_bar);
                let value = ui::handle_confirm(&message, default.unwrap_or(false))?;
                send_response(child, &id, serde_json::Value::Bool(value))?;
            }
            OutboundMessage::Select {
                id,
                message,
                options,
                default,
            } => {
                ui::clear_progress(&mut progress_bar);
                let value = ui::handle_select(&message, &options, default)?;
                send_response(child, &id, serde_json::Value::String(value))?;
            }
            OutboundMessage::MultiSelect {
                id,
                message,
                options,
                defaults,
            } => {
                ui::clear_progress(&mut progress_bar);
                let values = ui::handle_multi_select(&message, &options, defaults.as_deref())?;
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
                ui::handle_progress(
                    &mut progress_bar,
                    plugin_name,
                    message.as_deref(),
                    current,
                    total,
                    done.unwrap_or(false),
                );
            }
            OutboundMessage::Log { level, message } => {
                ui::clear_progress(&mut progress_bar);
                ui::handle_log(plugin_name, &level, &message);
            }
            OutboundMessage::Output { text } => {
                ui::clear_progress(&mut progress_bar);
                print!("{}", text);
            }
            OutboundMessage::Store { key, value } => {
                if !capabilities.store {
                    continue;
                }
                store::handle_store(plugin_dir, &key, &value)?;
            }
            OutboundMessage::Load { id, key } => {
                if !capabilities.store {
                    send_response(child, &id, serde_json::Value::Null)?;
                    continue;
                }
                let value = store::handle_load(plugin_dir, &key)?;
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
                let result = exec::handle_exec(&command, cwd.as_deref(), timeout, plugin_dir)?;
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
                let result = metadata::handle_metadata(&keys)?;
                send_response(child, &id, result)?;
            }
        }
    }

    ui::clear_progress(&mut progress_bar);
    Ok(())
}

pub(super) fn send_message<T: Serialize>(child: &mut Child, msg: &T) -> Result<()> {
    send_raw(child, msg)
}

pub(super) fn send_raw<T: Serialize>(child: &mut Child, msg: &T) -> Result<()> {
    let stdin = child.stdin.as_mut().context("plugin stdin unavailable")?;
    let json = serde_json::to_string(msg).context("serializing message")?;
    writeln!(stdin, "{}", json).context("writing to plugin stdin")?;
    stdin.flush().context("flushing plugin stdin")?;
    Ok(())
}

pub(super) fn send_response(child: &mut Child, id: &str, value: serde_json::Value) -> Result<()> {
    send_raw(
        child,
        &InboundResponse {
            msg_type: "response",
            id: id.to_string(),
            value,
        },
    )
}
