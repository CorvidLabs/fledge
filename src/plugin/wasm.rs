use anyhow::{bail, Context, Result};
use console::style;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

use crate::plugin::PluginCapabilities;

const FUEL_LIMIT: u64 = 10_000_000_000;
const WALL_CLOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

struct HostState {
    wasi: WasiP1Ctx,
    plugin_name: String,
    plugin_dir: PathBuf,
    #[allow(dead_code)]
    capabilities: PluginCapabilities,
    pending_response: Option<Vec<u8>>,
}

fn create_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Engine::new(&config).context("creating Wasmtime engine")
}

pub(super) fn load_module(engine: &Engine, wasm_path: &Path) -> Result<Module> {
    let cwasm_path = wasm_path.with_extension("cwasm");
    if cwasm_path.exists() && is_cache_valid(wasm_path, &cwasm_path)? {
        unsafe {
            Module::deserialize_file(engine, &cwasm_path).context("loading cached WASM module")
        }
    } else {
        Module::from_file(engine, wasm_path).context("compiling WASM module")
    }
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

#[allow(dead_code)]
pub(super) fn compile_and_cache(wasm_path: &Path) -> Result<()> {
    let engine = create_engine()?;
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("reading WASM binary: {}", wasm_path.display()))?;
    let module = Module::new(&engine, &wasm_bytes)
        .with_context(|| format!("compiling WASM module: {}", wasm_path.display()))?;

    let cwasm_path = wasm_path.with_extension("cwasm");
    let serialized = module.serialize().context("serializing compiled module")?;
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

fn setup_linker(engine: &Engine, capabilities: &PluginCapabilities) -> Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |s: &mut HostState| &mut s.wasi)?;

    linker.func_wrap(
        "fledge",
        "send",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<()> {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .context("plugin has no memory export")?;
            let data = memory.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end > data.len() {
                bail!("send: out-of-bounds memory access");
            }
            let msg_bytes = data[start..end].to_vec();
            handle_outbound_json(&caller, &msg_bytes);
            Ok(())
        },
    )?;

    linker.func_wrap(
        "fledge",
        "recv",
        |mut caller: Caller<'_, HostState>, ptr: i32, max_len: i32| -> Result<i32> {
            let response = caller
                .data_mut()
                .pending_response
                .take()
                .unwrap_or_default();
            let len = response.len().min(max_len as usize);
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .context("plugin has no memory export")?;
            memory.write(&mut caller, ptr as usize, &response[..len])?;
            Ok(len as i32)
        },
    )?;

    linker.func_wrap(
        "fledge",
        "exit",
        |_caller: Caller<'_, HostState>, code: i32| -> Result<()> {
            if code == 0 {
                bail!("__fledge_exit_ok__");
            }
            bail!("plugin exited with code {}", code);
        },
    )?;

    if capabilities.exec {
        linker.func_wrap(
            "fledge",
            "exec",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<i32> {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .context("plugin has no memory export")?;
                let data = memory.data(&caller);
                let request: serde_json::Value =
                    serde_json::from_slice(&data[ptr as usize..(ptr as usize + len as usize)])?;

                let command = request["command"].as_str().unwrap_or_default();
                let cwd = request["cwd"].as_str();
                let timeout = request["timeout"].as_u64();
                let plugin_dir = caller.data().plugin_dir.clone();

                let result =
                    crate::protocol::exec::handle_exec(command, cwd, timeout, &plugin_dir)?;
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
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<()> {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .context("plugin has no memory export")?;
                let data = memory.data(&caller);
                let request: serde_json::Value =
                    serde_json::from_slice(&data[ptr as usize..(ptr as usize + len as usize)])?;
                let key = request["key"].as_str().unwrap_or_default();
                let value = request["value"].as_str().unwrap_or_default();
                let plugin_dir = caller.data().plugin_dir.clone();
                crate::protocol::store::handle_store(&plugin_dir, key, value)?;
                Ok(())
            },
        )?;

        linker.func_wrap(
            "fledge",
            "store_get",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<i32> {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .context("plugin has no memory export")?;
                let data = memory.data(&caller);
                let key = std::str::from_utf8(&data[ptr as usize..(ptr as usize + len as usize)])?;
                let plugin_dir = caller.data().plugin_dir.clone();
                let value = crate::protocol::store::handle_load(&plugin_dir, key)?;
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
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<i32> {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .context("plugin has no memory export")?;
                let data = memory.data(&caller);
                let request: serde_json::Value =
                    serde_json::from_slice(&data[ptr as usize..(ptr as usize + len as usize)])?;
                let keys: Vec<String> = request
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let result = crate::protocol::metadata::handle_metadata(&keys)?;
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
    let msg: crate::protocol::OutboundMessage = match serde_json::from_slice(msg_bytes) {
        Ok(m) => m,
        Err(_) => return,
    };
    match msg {
        crate::protocol::OutboundMessage::Output { text } => {
            print!("{}", text);
        }
        crate::protocol::OutboundMessage::Log { level, message } => {
            crate::protocol::ui::handle_log(plugin_name, &level, &message);
        }
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

    let host_state = HostState {
        wasi,
        plugin_name: plugin_name.to_string(),
        plugin_dir: plugin_dir.to_path_buf(),
        capabilities: capabilities.clone(),
        pending_response: None,
    };

    let mut store = Store::new(&engine, host_state);
    store.set_fuel(FUEL_LIMIT)?;

    let engine_clone = engine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(WALL_CLOCK_TIMEOUT);
        engine_clone.increment_epoch();
    });
    store.epoch_deadline_trap();
    store.set_epoch_deadline(1);

    let linker = setup_linker(&engine, capabilities)?;

    println!(
        "  {} Running WASM plugin {}",
        style("▶").cyan().bold(),
        style(plugin_name).cyan()
    );

    let project_ctx = crate::protocol::detect::detect_project_context();
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
        },
    };
    let init_bytes = serde_json::to_vec(&init_msg)?;
    store.data_mut().pending_response = Some(init_bytes);

    let instance = linker.instantiate(&mut store, &module).context(
        "instantiating WASM plugin — the plugin may import functions for capabilities it wasn't granted",
    )?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .context("WASM module has no _start export")?;

    match start.call(&mut store, ()) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("__fledge_exit_ok__") {
                Ok(())
            } else if msg.contains("all fuel consumed") {
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
