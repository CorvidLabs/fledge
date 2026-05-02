use anyhow::{bail, Context, Result};
use console::style;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

use crate::plugin::PluginCapabilities;

const FUEL_LIMIT: u64 = 10_000_000_000;
const WALL_CLOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug)]
struct FledgeExitOk;

impl std::fmt::Display for FledgeExitOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "plugin exited successfully")
    }
}

impl std::error::Error for FledgeExitOk {}

struct HostState {
    wasi: WasiP1Ctx,
    plugin_name: String,
    plugin_dir: PathBuf,
    capabilities: PluginCapabilities,
    pending_response: Option<Vec<u8>>,
    limits: StoreLimits,
}

fn create_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Ok(Engine::new(&config)?)
}

pub(super) fn load_module(engine: &Engine, wasm_path: &Path) -> Result<Module> {
    let cwasm_path = wasm_path.with_extension("cwasm");
    if cwasm_path.exists() && is_cache_valid(wasm_path, &cwasm_path)? {
        match unsafe { Module::deserialize_file(engine, &cwasm_path) } {
            Ok(module) => return Ok(module),
            Err(_) => {
                let _ = std::fs::remove_file(&cwasm_path);
                let _ = std::fs::remove_file(cwasm_path.with_extension("cwasm.sha256"));
            }
        }
    }
    let module = Module::from_file(engine, wasm_path)?;
    if let Ok(serialized) = module.serialize() {
        let _ = std::fs::write(&cwasm_path, &serialized);
        if let Ok(wasm_bytes) = std::fs::read(wasm_path) {
            let hash = compute_hash(&wasm_bytes);
            let _ = std::fs::write(cwasm_path.with_extension("cwasm.sha256"), &hash);
        }
    }
    Ok(module)
}

fn is_cache_valid(wasm_path: &Path, cwasm_path: &Path) -> Result<bool> {
    let wasm_bytes = std::fs::read(wasm_path)?;
    let expected_hash = compute_hash(&wasm_bytes);
    let hash_path = cwasm_path.with_extension("cwasm.sha256");
    match std::fs::read_to_string(&hash_path) {
        Ok(stored) => Ok(stored.trim() == expected_hash),
        Err(_) => Ok(false),
    }
}

fn compute_hash(data: &[u8]) -> String {
    let result = Sha256::digest(data);
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

pub(super) fn compile_and_cache(wasm_path: &Path) -> Result<()> {
    let engine = create_engine()?;
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("reading WASM binary: {}", wasm_path.display()))?;
    let module = Module::new(&engine, &wasm_bytes)
        .map_err(|e| anyhow::anyhow!("compiling WASM module {}: {e}", wasm_path.display()))?;

    let cwasm_path = wasm_path.with_extension("cwasm");
    let serialized = module.serialize()?;
    std::fs::write(&cwasm_path, &serialized)
        .with_context(|| format!("writing cached module: {}", cwasm_path.display()))?;

    let hash = compute_hash(&wasm_bytes);
    let hash_path = cwasm_path.with_extension("cwasm.sha256");
    std::fs::write(&hash_path, &hash)
        .with_context(|| format!("writing module hash: {}", hash_path.display()))?;

    Ok(())
}

fn build_wasi_p1(
    capabilities: &PluginCapabilities,
    plugin_dir: &Path,
    project_root: Option<&Path>,
) -> Result<WasiP1Ctx> {
    let mut builder = WasiCtxBuilder::new();

    builder.inherit_stderr();

    match capabilities.filesystem.as_deref() {
        Some("project") => {
            if let Some(root) = project_root {
                builder.preopened_dir(root, "/project", DirPerms::READ, FilePerms::READ)?;
            }
        }
        Some("plugin") => {
            if let Some(root) = project_root {
                builder.preopened_dir(root, "/project", DirPerms::READ, FilePerms::READ)?;
            }
            builder.preopened_dir(plugin_dir, "/plugin", DirPerms::all(), FilePerms::all())?;
        }
        _ => {}
    }

    if capabilities.network {
        builder.inherit_network();
    }

    Ok(builder.build_p1())
}

fn get_memory(caller: &mut Caller<'_, HostState>) -> std::result::Result<Memory, wasmtime::Error> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| wasmtime::Error::msg("plugin has no memory export"))
}

fn validate_guest_range(
    ptr: i32,
    len: i32,
) -> std::result::Result<(usize, usize), wasmtime::Error> {
    if ptr < 0 || len < 0 {
        return Err(wasmtime::Error::msg(
            "negative pointer or length from guest",
        ));
    }
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .ok_or_else(|| wasmtime::Error::msg("guest memory range overflow"))?;
    Ok((start, end))
}

fn read_guest_slice(
    data: &[u8],
    ptr: i32,
    len: i32,
) -> std::result::Result<&[u8], wasmtime::Error> {
    let (start, end) = validate_guest_range(ptr, len)?;
    if end > data.len() {
        return Err(wasmtime::Error::msg("out-of-bounds guest memory access"));
    }
    Ok(&data[start..end])
}

fn setup_linker(engine: &Engine, capabilities: &PluginCapabilities) -> Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s: &mut HostState| &mut s.wasi)?;

    linker.func_wrap(
        "fledge",
        "send",
        |mut caller: Caller<'_, HostState>,
         ptr: i32,
         len: i32|
         -> std::result::Result<(), wasmtime::Error> {
            let memory = get_memory(&mut caller)?;
            let data = memory.data(&caller);
            let msg_bytes = read_guest_slice(data, ptr, len)?.to_vec();
            handle_outbound_json(&caller, &msg_bytes);
            Ok(())
        },
    )?;

    linker.func_wrap(
        "fledge",
        "recv",
        |mut caller: Caller<'_, HostState>,
         ptr: i32,
         max_len: i32|
         -> std::result::Result<i32, wasmtime::Error> {
            let (start, _) = validate_guest_range(ptr, max_len)?;
            let response = caller
                .data_mut()
                .pending_response
                .take()
                .unwrap_or_default();
            let len = response.len().min(max_len as usize);
            let memory = get_memory(&mut caller)?;
            memory.write(&mut caller, start, &response[..len])?;
            if response.len() > len {
                caller.data_mut().pending_response = Some(response[len..].to_vec());
            }
            Ok(len as i32)
        },
    )?;

    linker.func_wrap(
        "fledge",
        "exit",
        |_caller: Caller<'_, HostState>, code: i32| -> std::result::Result<(), wasmtime::Error> {
            if code == 0 {
                return Err(wasmtime::Error::from_anyhow(anyhow::Error::new(
                    FledgeExitOk,
                )));
            }
            Err(wasmtime::Error::msg(format!(
                "plugin exited with code {}",
                code
            )))
        },
    )?;

    if capabilities.exec {
        linker.func_wrap(
            "fledge",
            "exec",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             len: i32|
             -> std::result::Result<i32, wasmtime::Error> {
                let memory = get_memory(&mut caller)?;
                let data = memory.data(&caller);
                let slice = read_guest_slice(data, ptr, len)?;
                let request: serde_json::Value = serde_json::from_slice(slice)?;

                let command = request["command"].as_str().unwrap_or_default();
                let cwd = request["cwd"].as_str();
                let timeout = request["timeout"].as_u64();
                let plugin_dir = caller.data().plugin_dir.clone();

                let result = crate::protocol::handle_exec(command, cwd, timeout, &plugin_dir)
                    .map_err(wasmtime::Error::from_anyhow)?;
                let result_bytes = serde_json::to_vec(&result)?;
                let result_len = result_bytes.len();
                caller.data_mut().pending_response = Some(result_bytes);
                Ok(result_len as i32)
            },
        )?;
    }

    if capabilities.store {
        linker.func_wrap(
            "fledge",
            "store_set",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             len: i32|
             -> std::result::Result<(), wasmtime::Error> {
                let memory = get_memory(&mut caller)?;
                let data = memory.data(&caller);
                let slice = read_guest_slice(data, ptr, len)?;
                let request: serde_json::Value = serde_json::from_slice(slice)?;
                let key = request["key"].as_str().unwrap_or_default();
                let value = request["value"].as_str().unwrap_or_default();
                let plugin_dir = caller.data().plugin_dir.clone();
                crate::protocol::handle_store(&plugin_dir, key, value)
                    .map_err(wasmtime::Error::from_anyhow)?;
                Ok(())
            },
        )?;

        linker.func_wrap(
            "fledge",
            "store_get",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             len: i32|
             -> std::result::Result<i32, wasmtime::Error> {
                let memory = get_memory(&mut caller)?;
                let data = memory.data(&caller);
                let slice = read_guest_slice(data, ptr, len)?;
                let key = std::str::from_utf8(slice)?;
                let plugin_dir = caller.data().plugin_dir.clone();
                let value = crate::protocol::handle_load(&plugin_dir, key)
                    .map_err(wasmtime::Error::from_anyhow)?;
                let value_bytes = serde_json::to_vec(&value)?;
                let value_len = value_bytes.len();
                caller.data_mut().pending_response = Some(value_bytes);
                Ok(value_len as i32)
            },
        )?;
    }

    if capabilities.metadata {
        linker.func_wrap(
            "fledge",
            "metadata",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             len: i32|
             -> std::result::Result<i32, wasmtime::Error> {
                let memory = get_memory(&mut caller)?;
                let data = memory.data(&caller);
                let slice = read_guest_slice(data, ptr, len)?;
                let request: serde_json::Value = serde_json::from_slice(slice)?;
                let keys: Vec<String> = request
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let result = crate::protocol::handle_metadata(&keys)
                    .map_err(wasmtime::Error::from_anyhow)?;
                let result_bytes = serde_json::to_vec(&result)?;
                let result_len = result_bytes.len();
                caller.data_mut().pending_response = Some(result_bytes);
                Ok(result_len as i32)
            },
        )?;
    }

    Ok(linker)
}

fn handle_outbound_json(caller: &Caller<'_, HostState>, msg_bytes: &[u8]) {
    let plugin_name = &caller.data().plugin_name;
    let plugin_dir = &caller.data().plugin_dir;
    let capabilities = &caller.data().capabilities;
    let msg: crate::protocol::OutboundMessage = match serde_json::from_slice(msg_bytes) {
        Ok(m) => m,
        Err(_) => return,
    };
    match msg {
        crate::protocol::OutboundMessage::Output { text } => {
            print!("{}", text);
        }
        crate::protocol::OutboundMessage::Log { level, message } => {
            crate::protocol::handle_log(plugin_name, &level, &message);
        }
        crate::protocol::OutboundMessage::Progress {
            message: Some(msg),
            current,
            total,
            done,
        } => {
            let pct = match (current, total) {
                (Some(c), Some(t)) if t > 0 => format!(" ({}/{})", c, t),
                _ => String::new(),
            };
            let done_mark = if done.unwrap_or(false) { " done" } else { "" };
            eprintln!(
                "  {} [{}] {}{}{}",
                style("▪").dim(),
                style(plugin_name).dim(),
                msg,
                pct,
                done_mark
            );
        }
        crate::protocol::OutboundMessage::Store { key, value } => {
            if capabilities.store {
                if let Err(e) = crate::protocol::handle_store(plugin_dir, &key, &value) {
                    eprintln!(
                        "  {} [{}] store rejected: {}",
                        style("⚠").yellow(),
                        plugin_name,
                        e
                    );
                }
            }
        }
        crate::protocol::OutboundMessage::Prompt { .. }
        | crate::protocol::OutboundMessage::Confirm { .. }
        | crate::protocol::OutboundMessage::Select { .. }
        | crate::protocol::OutboundMessage::MultiSelect { .. } => {
            eprintln!(
                "  {} [{}] interactive UI messages are not supported in WASM sandbox mode",
                style("⚠").yellow(),
                plugin_name
            );
        }
        // Load, Exec, and Metadata messages are handled via dedicated host
        // imports in WASM mode, not through fledge::send(). If a plugin
        // sends them via send() anyway, ignore them silently.
        _ => {}
    }
}

pub(super) fn run_wasm_plugin(
    wasm_path: &Path,
    args: &[String],
    plugin_name: &str,
    plugin_version: &str,
    plugin_dir: &Path,
    capabilities: &PluginCapabilities,
) -> Result<()> {
    let engine = create_engine()?;
    let module = load_module(&engine, wasm_path)?;

    let project_root = std::env::current_dir().ok();
    let wasi = build_wasi_p1(capabilities, plugin_dir, project_root.as_deref())?;

    let limits = StoreLimitsBuilder::new()
        .memory_size(MAX_MEMORY_BYTES)
        .build();

    let host_state = HostState {
        wasi,
        plugin_name: plugin_name.to_string(),
        plugin_dir: plugin_dir.to_path_buf(),
        capabilities: capabilities.clone(),
        pending_response: None,
        limits,
    };

    let mut store = Store::new(&engine, host_state);
    store.limiter(|s| &mut s.limits);
    store.set_fuel(FUEL_LIMIT)?;

    let finished = Arc::new(AtomicBool::new(false));
    let finished_clone = finished.clone();
    let engine_clone = engine.clone();
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + WALL_CLOCK_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() || finished_clone.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(remaining.min(std::time::Duration::from_millis(250)));
        }
        if !finished_clone.load(Ordering::Relaxed) {
            engine_clone.increment_epoch();
        }
    });
    store.epoch_deadline_trap();
    store.set_epoch_deadline(1);

    let linker = setup_linker(&engine, capabilities)?;

    println!(
        "  {} Running WASM plugin {}",
        style("▶").cyan().bold(),
        style(plugin_name).cyan()
    );

    let project_ctx = crate::protocol::detect_project_context();
    let init_msg = crate::protocol::PluginContext {
        msg_type: "init",
        protocol: "fledge-v1",
        args: args.to_vec(),
        project: project_ctx,
        plugin: crate::protocol::PluginInfo {
            name: plugin_name.to_string(),
            version: plugin_version.to_string(),
            dir: plugin_dir.to_string_lossy().to_string(),
        },
        fledge: crate::protocol::FledgeInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        capabilities: crate::protocol::CapabilitiesInfo {
            exec: capabilities.exec,
            store: capabilities.store,
            metadata: capabilities.metadata,
            filesystem: capabilities.filesystem.clone(),
            network: if capabilities.network {
                Some(true)
            } else {
                None
            },
        },
    };
    let init_bytes = serde_json::to_vec(&init_msg)?;
    store.data_mut().pending_response = Some(init_bytes);

    let instance = linker.instantiate(&mut store, &module).map_err(|e| {
        anyhow::anyhow!("instantiating WASM plugin — the plugin may import functions for capabilities it wasn't granted: {e}")
    })?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| anyhow::anyhow!("WASM module has no _start export: {e}"))?;

    let result = start.call(&mut store, ());
    finished.store(true, Ordering::Relaxed);

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.downcast_ref::<FledgeExitOk>().is_some() {
                Ok(())
            } else {
                let msg = e.to_string();
                if msg.contains("all fuel consumed") {
                    bail!(
                        "Plugin '{}' exceeded compute limit (fuel exhausted)",
                        plugin_name
                    )
                } else if msg.contains("epoch") {
                    bail!(
                        "Plugin '{}' exceeded time limit ({} seconds)",
                        plugin_name,
                        WALL_CLOCK_TIMEOUT.as_secs()
                    )
                } else {
                    bail!("Plugin '{}' trapped: {}", plugin_name, e)
                }
            }
        }
    }
}
