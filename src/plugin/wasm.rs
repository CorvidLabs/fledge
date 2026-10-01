use anyhow::{bail, Context, Result};
use console::style;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{FsPerms, WasiCtxBuilder};

use crate::plugin::PluginCapabilities;

const FUEL_LIMIT: u64 = 10_000_000_000;
const WALL_CLOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;
const WASMTIME_VERSION: &str = env!("WASMTIME_DEP_VERSION");

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

pub(crate) fn load_module(engine: &Engine, wasm_path: &Path) -> Result<Module> {
    let wasm_bytes = std::fs::read(wasm_path)?;
    let cwasm_path = wasm_path.with_extension("cwasm");
    if cwasm_path.exists() {
        if let Some(module) = try_load_cached(engine, &wasm_bytes, &cwasm_path)? {
            return Ok(module);
        }
    }
    let module = Module::new(engine, &wasm_bytes)?;
    if let Ok(serialized) = module.serialize() {
        let tmp_cwasm = cwasm_path.with_extension("cwasm.tmp");
        if std::fs::write(&tmp_cwasm, &serialized).is_ok()
            && std::fs::rename(&tmp_cwasm, &cwasm_path).is_ok()
        {
            let wasm_hash = compute_hash(&wasm_bytes);
            let cwasm_hash = compute_hash(&serialized);
            let stamp = format!("{}\n{}\n{}", wasm_hash, WASMTIME_VERSION, cwasm_hash);
            let hash_path = cwasm_path.with_extension("cwasm.sha256");
            let tmp_hash = hash_path.with_extension("sha256.tmp");
            if std::fs::write(&tmp_hash, &stamp).is_ok() {
                let _ = std::fs::rename(&tmp_hash, &hash_path);
            }
        }
    }
    Ok(module)
}

fn try_load_cached(
    engine: &Engine,
    wasm_bytes: &[u8],
    cwasm_path: &Path,
) -> Result<Option<Module>> {
    let expected_wasm_hash = compute_hash(wasm_bytes);
    let cwasm_bytes = match std::fs::read(cwasm_path) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let actual_cwasm_hash = compute_hash(&cwasm_bytes);
    let hash_path = cwasm_path.with_extension("cwasm.sha256");
    let stamp = match std::fs::read_to_string(&hash_path) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let mut lines = stamp.lines();
    let wasm_ok = lines.next().is_some_and(|h| h.trim() == expected_wasm_hash);
    let version_ok = lines.next().is_some_and(|v| v.trim() == WASMTIME_VERSION);
    let cwasm_ok = lines.next().is_some_and(|h| h.trim() == actual_cwasm_hash);
    if !(wasm_ok && version_ok && cwasm_ok) {
        return Ok(None);
    }
    match unsafe { Module::deserialize(engine, &cwasm_bytes) } {
        Ok(module) => Ok(Some(module)),
        Err(e) => {
            eprintln!(
                "  {} cached module invalid (recompiling): {}",
                style("⚠").yellow(),
                e
            );
            let _ = std::fs::remove_file(cwasm_path);
            let _ = std::fs::remove_file(&hash_path);
            Ok(None)
        }
    }
}

fn compute_hash(data: &[u8]) -> String {
    let result = Sha256::digest(data);
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

pub(crate) fn compile_and_cache(wasm_path: &Path) -> Result<()> {
    let engine = create_engine()?;
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("reading WASM binary: {}", wasm_path.display()))?;
    let module = Module::new(&engine, &wasm_bytes)
        .map_err(|e| anyhow::anyhow!("compiling WASM module {}: {e}", wasm_path.display()))?;

    let cwasm_path = wasm_path.with_extension("cwasm");
    let serialized = module.serialize()?;

    // Atomic write: temp file + rename to avoid races on concurrent installs
    let tmp_cwasm = cwasm_path.with_extension("cwasm.tmp");
    std::fs::write(&tmp_cwasm, &serialized)
        .with_context(|| format!("writing cached module: {}", cwasm_path.display()))?;
    std::fs::rename(&tmp_cwasm, &cwasm_path)
        .with_context(|| format!("finalizing cached module: {}", cwasm_path.display()))?;

    let wasm_hash = compute_hash(&wasm_bytes);
    let cwasm_hash = compute_hash(&serialized);
    let hash_path = cwasm_path.with_extension("cwasm.sha256");
    let stamp = format!("{}\n{}\n{}", wasm_hash, WASMTIME_VERSION, cwasm_hash);
    let tmp_hash = hash_path.with_extension("sha256.tmp");
    std::fs::write(&tmp_hash, &stamp)
        .with_context(|| format!("writing module hash: {}", hash_path.display()))?;
    std::fs::rename(&tmp_hash, &hash_path)
        .with_context(|| format!("finalizing module hash: {}", hash_path.display()))?;

    Ok(())
}

fn build_wasi_p1(
    capabilities: &PluginCapabilities,
    plugin_dir: &Path,
    project_root: Option<&Path>,
) -> Result<WasiP1Ctx> {
    let mut builder = WasiCtxBuilder::new();

    let resolved_plugin_dir = plugin_dir
        .canonicalize()
        .unwrap_or_else(|_| plugin_dir.to_path_buf());

    // `/project` is read-only (no create, remove, rename or write-open);
    // `/plugin` is read-write. wasmtime-wasi 49 replaced the separate
    // DirPerms/FilePerms bitflags with `FsPerms`: `ReadOnly` is the old
    // `DirPerms::READ` + `FilePerms::READ`, `ReadWrite` the old `all()` + `all()`.
    match capabilities.filesystem.as_deref() {
        Some("project") => {
            if let Some(root) = project_root {
                let resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
                builder.preopened_dir(&resolved, "/project", FsPerms::ReadOnly)?;
            }
        }
        Some("plugin") => {
            if let Some(root) = project_root {
                let resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
                builder.preopened_dir(&resolved, "/project", FsPerms::ReadOnly)?;
            }
            let data_dir = resolved_plugin_dir.join("data");
            std::fs::create_dir_all(&data_dir)?;
            builder.preopened_dir(&data_dir, "/plugin", FsPerms::ReadWrite)?;
        }
        _ => {}
    }

    builder.inherit_stdout();

    if capabilities.network {
        // wasmtime-wasi 49 turned TCP and UDP off by default (46 had them on),
        // so `inherit_network()`, which only opens the socket address check,
        // no longer grants sockets by itself. Re-enable both so a plugin
        // granted `network` keeps exactly what it had. IP name lookup stays
        // off, as before. Without the grant, sockets stay disabled.
        builder.inherit_network().allow_tcp(true).allow_udp(true);
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
                let request: serde_json::Value = serde_json::from_slice(slice)
                    .map_err(|e| wasmtime::Error::msg(format!("exec: malformed JSON: {e}")))?;

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
                let request: serde_json::Value = serde_json::from_slice(slice)
                    .map_err(|e| wasmtime::Error::msg(format!("store_set: malformed JSON: {e}")))?;
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
                let request: serde_json::Value = serde_json::from_slice(slice)
                    .map_err(|e| wasmtime::Error::msg(format!("metadata: malformed JSON: {e}")))?;
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
        Err(e) => {
            eprintln!(
                "  {} [{}] malformed message (dropped): {}",
                style("⚠").yellow(),
                style(plugin_name).dim(),
                e
            );
            return;
        }
    };
    match msg {
        crate::protocol::OutboundMessage::Output { text } => {
            print!("{}", text);
            let _ = std::io::Write::flush(&mut std::io::stdout());
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
        crate::protocol::OutboundMessage::Store { key, value } if capabilities.store => {
            if let Err(e) = crate::protocol::handle_store(plugin_dir, &key, &value) {
                eprintln!(
                    "  {} [{}] store rejected: {}",
                    style("⚠").yellow(),
                    plugin_name,
                    e
                );
            }
        }
        crate::protocol::OutboundMessage::Store { .. } => {
            eprintln!(
                "  {} [{}] store not granted — message dropped",
                style("⚠").yellow(),
                plugin_name
            );
        }
        crate::protocol::OutboundMessage::Prompt { .. }
        | crate::protocol::OutboundMessage::Confirm { .. }
        | crate::protocol::OutboundMessage::Select { .. }
        | crate::protocol::OutboundMessage::MultiSelect { .. } => {
            eprintln!(
                "  {} [{}] interactive UI (prompt/confirm/select) not supported in WASM mode",
                style("⚠").yellow(),
                plugin_name
            );
        }
        // Load, Exec, Metadata — handled via dedicated host functions, not outbound JSON
        _ => {}
    }
}

pub(crate) fn run_wasm_plugin(
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
    let timeout_handle = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + WALL_CLOCK_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() || finished_clone.load(Ordering::Acquire) {
                break;
            }
            std::thread::sleep(remaining.min(std::time::Duration::from_millis(250)));
        }
        if !finished_clone.load(Ordering::Acquire) {
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
        let err_msg = e.to_string();
        let cap_hints: Vec<&str> = [
            ("exec", "exec", capabilities.exec),
            ("store_set", "store", capabilities.store),
            ("store_get", "store", capabilities.store),
            ("metadata", "metadata", capabilities.metadata),
        ]
        .iter()
        .filter(|(import, _, granted)| !granted && err_msg.contains(import))
        .map(|(_, cap, _)| *cap)
        .collect();
        if !cap_hints.is_empty() {
            let caps = cap_hints
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            anyhow::anyhow!(
                "WASM plugin imports functions that require capabilities not granted: {}\n  \
                 Add these to [capabilities] in plugin.toml and reinstall.",
                caps.into_iter().collect::<Vec<_>>().join(", ")
            )
        } else {
            anyhow::anyhow!("Failed to instantiate WASM plugin: {e}")
        }
    })?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| anyhow::anyhow!("WASM module has no _start export: {e}"))?;

    let result = start.call(&mut store, ());
    finished.store(true, Ordering::Release);
    let _ = timeout_handle.join();

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.downcast_ref::<FledgeExitOk>().is_some() {
                return Ok(());
            }
            if let Some(trap) = e.downcast_ref::<Trap>() {
                match trap {
                    Trap::OutOfFuel => bail!(
                        "Plugin '{}' exceeded its compute budget.\n  \
                         The plugin ran too many instructions. This is a safety limit, not a bug in fledge.\n  \
                         If the plugin is doing heavy work, it may need to be split into smaller steps.",
                        plugin_name
                    ),
                    Trap::Interrupt => bail!(
                        "Plugin '{}' exceeded time limit ({} seconds)",
                        plugin_name,
                        WALL_CLOCK_TIMEOUT.as_secs()
                    ),
                    _ => bail!("Plugin '{}' trapped: {}", plugin_name, trap),
                }
            }
            bail!("Plugin '{}' failed: {}", plugin_name, e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal WASM module: imports fledge protocol, calls exit(0) immediately
    const EXIT_OK_WAT: &str = r#"(module
        (import "fledge" "exit" (func $exit (param i32)))
        (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
        (import "fledge" "send" (func $send (param i32 i32)))
        (memory (export "memory") 1)
        (func (export "_start") (call $exit (i32.const 0)))
    )"#;

    #[test]
    fn engine_creates_successfully() {
        create_engine().unwrap();
    }

    #[test]
    fn compute_hash_deterministic() {
        let h1 = compute_hash(b"hello world");
        let h2 = compute_hash(b"hello world");
        assert_eq!(h1, h2);
        assert_ne!(h1, compute_hash(b"different input"));
    }

    #[test]
    fn cache_stores_wasmtime_version_stamp() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let hash_path = cwasm_path.with_extension("cwasm.sha256");
        let contents = std::fs::read_to_string(&hash_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "hash file should have wasm_hash + version + cwasm_hash"
        );
        assert_eq!(lines[1], WASMTIME_VERSION);
    }

    #[test]
    fn cache_valid_with_matching_version() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        assert!(cwasm_path.exists());
        let engine = create_engine().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        assert!(try_load_cached(&engine, &wasm_bytes, &cwasm_path)
            .unwrap()
            .is_some());
    }

    #[test]
    fn cache_invalid_on_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let hash_path = cwasm_path.with_extension("cwasm.sha256");
        let contents = std::fs::read_to_string(&hash_path).unwrap();
        let mut lines = contents.lines();
        let hash_line = lines.next().unwrap();
        let _version = lines.next().unwrap();
        let cwasm_hash = lines.next().unwrap();
        std::fs::write(&hash_path, format!("{}\n999\n{}", hash_line, cwasm_hash)).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let engine = create_engine().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        assert!(try_load_cached(&engine, &wasm_bytes, &cwasm_path)
            .unwrap()
            .is_none());
    }

    #[test]
    fn cache_invalid_on_hash_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let modified = format!("{}\n;; modified", EXIT_OK_WAT);
        std::fs::write(&wasm_path, &modified).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let engine = create_engine().unwrap();
        let wasm_bytes = modified.as_bytes();
        assert!(try_load_cached(&engine, wasm_bytes, &cwasm_path)
            .unwrap()
            .is_none());
    }

    #[test]
    fn load_module_compiles_from_source() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        let engine = create_engine().unwrap();
        let module = load_module(&engine, &wasm_path).unwrap();
        assert!(module.get_export("_start").is_some());
    }

    #[test]
    fn load_module_uses_valid_cache() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let engine = create_engine().unwrap();
        let module = load_module(&engine, &wasm_path).unwrap();
        assert!(module.get_export("_start").is_some());
    }

    #[test]
    fn run_wasm_plugin_exit_ok() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-runtime", "0.0.1", dir.path(), &caps);
        assert!(result.is_ok(), "run_wasm_plugin should succeed: {result:?}");
    }

    // Verifies build.rs actually ran and emitted WASMTIME_DEP_VERSION.
    #[test]
    fn wasmtime_version_derived_from_cargo_toml() {
        let cargo_toml = include_str!("../../Cargo.toml");
        let parsed: toml::Value = cargo_toml.parse().unwrap();
        let wt = &parsed["dependencies"]["wasmtime"];
        let dep_version = wt
            .as_str()
            .or_else(|| wt.get("version").and_then(|v| v.as_str()))
            .expect("wasmtime dependency should have a version");
        assert_eq!(
            WASMTIME_VERSION, dep_version,
            "build.rs-derived WASMTIME_VERSION ({}) doesn't match Cargo.toml wasmtime = \"{}\"",
            WASMTIME_VERSION, dep_version
        );
    }

    #[test]
    fn fuel_exhaustion_returns_error() {
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "_start") (loop $spin (br $spin)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("spin.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-spin", "0.0.1", dir.path(), &caps);
        assert!(result.is_err(), "infinite loop should be terminated");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("compute budget") || err.contains("time limit") || err.contains("trapped"),
            "expected resource-limit error, got: {err}"
        );
    }

    #[test]
    fn memory_is_capped_at_max_memory_bytes() {
        // The guest starts with one page and grows by `grow_pages`. It exits 0
        // only on the outcome the test expects and hits `unreachable` on the
        // other, so `Ok` means memory.grow returned what the cap requires.
        let run_grow = |grow_pages: usize, expect_refused: bool| {
            let (on_refused, on_granted) = if expect_refused {
                ("(call $exit (i32.const 0))", "(unreachable)")
            } else {
                ("(unreachable)", "(call $exit (i32.const 0))")
            };
            let wat = format!(
                r#"(module
                    (import "fledge" "exit" (func $exit (param i32)))
                    (memory (export "memory") 1)
                    (func (export "_start")
                        (if (i32.eq (memory.grow (i32.const {grow_pages})) (i32.const -1))
                            (then {on_refused})
                            (else {on_granted}))))"#
            );
            let dir = tempfile::tempdir().unwrap();
            let wasm_path = dir.path().join("grow.wasm");
            std::fs::write(&wasm_path, wat).unwrap();
            let caps = PluginCapabilities::default();
            run_wasm_plugin(&wasm_path, &[], "test-grow", "0.0.1", dir.path(), &caps)
        };
        let pages_at_cap = MAX_MEMORY_BYTES / (64 * 1024);

        let at_cap = run_grow(pages_at_cap - 1, false);
        assert!(
            at_cap.is_ok(),
            "growing to exactly the cap must succeed: {at_cap:?}"
        );

        let past_cap = run_grow(pages_at_cap, true);
        assert!(
            past_cap.is_ok(),
            "growing past the cap must be refused: {past_cap:?}"
        );
    }

    #[test]
    fn capability_denial_exec_traps() {
        // Module that tries to call exec — but caps.exec is false
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (import "fledge" "exec" (func $exec (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (func (export "_start") (call $exit (i32.const 0)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("exec_denied.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default(); // exec = false
        let result = run_wasm_plugin(&wasm_path, &[], "test-denied", "0.0.1", dir.path(), &caps);
        assert!(
            result.is_err(),
            "should fail when module imports exec but capability not granted"
        );
    }

    #[test]
    fn exit_nonzero_returns_error() {
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "_start") (call $exit (i32.const 1)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("exit_bad.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-exit-bad", "0.0.1", dir.path(), &caps);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exited with code 1") || err.contains("trapped") || err.contains("failed"),
            "expected non-zero exit error, got: {err}"
        );
    }

    #[test]
    fn cache_invalid_on_cwasm_tamper() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        std::fs::write(&cwasm_path, b"tampered data").unwrap();

        let engine = create_engine().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        assert!(
            try_load_cached(&engine, &wasm_bytes, &cwasm_path)
                .unwrap()
                .is_none(),
            "cache should be invalid after .cwasm tampering"
        );
    }

    #[test]
    fn malformed_wasm_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("bad.wasm");
        std::fs::write(&wasm_path, b"not a valid wasm module").unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-bad-wasm", "0.0.1", dir.path(), &caps);
        assert!(result.is_err(), "malformed WASM should fail to load");
    }

    // --- Guest memory validation ---

    #[test]
    fn validate_guest_range_rejects_negative_ptr() {
        let result = validate_guest_range(-1, 10);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("negative"),
            "should mention negative pointer"
        );
    }

    #[test]
    fn validate_guest_range_rejects_negative_len() {
        let result = validate_guest_range(0, -1);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("negative"),
            "should mention negative length"
        );
    }

    #[test]
    fn validate_guest_range_large_values_ok_on_64bit() {
        // On 64-bit, i32::MAX + i32::MAX fits in usize — overflow guard is for 32-bit
        let result = validate_guest_range(i32::MAX, i32::MAX);
        assert!(result.is_ok(), "should not overflow on 64-bit");
        let (start, end) = result.unwrap();
        assert_eq!(start, i32::MAX as usize);
        assert_eq!(end, (i32::MAX as usize) * 2);
    }

    #[test]
    fn validate_guest_range_accepts_valid_range() {
        let (start, end) = validate_guest_range(100, 50).unwrap();
        assert_eq!(start, 100);
        assert_eq!(end, 150);
    }

    #[test]
    fn validate_guest_range_accepts_zero_length() {
        let (start, end) = validate_guest_range(42, 0).unwrap();
        assert_eq!(start, 42);
        assert_eq!(end, 42);
    }

    #[test]
    fn read_guest_slice_rejects_out_of_bounds() {
        let data = vec![0u8; 100];
        let result = read_guest_slice(&data, 90, 20);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("out-of-bounds"),
            "should mention out-of-bounds"
        );
    }

    #[test]
    fn read_guest_slice_reads_valid_range() {
        let data: Vec<u8> = (0..100).collect();
        let slice = read_guest_slice(&data, 10, 5).unwrap();
        assert_eq!(slice, &[10, 11, 12, 13, 14]);
    }

    #[test]
    fn read_guest_slice_reads_exact_end() {
        let data = vec![0u8; 64];
        let slice = read_guest_slice(&data, 0, 64).unwrap();
        assert_eq!(slice.len(), 64);
    }

    // --- Cache edge cases ---

    #[test]
    fn cache_invalid_when_hash_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let hash_path = cwasm_path.with_extension("cwasm.sha256");
        std::fs::remove_file(&hash_path).unwrap();

        let engine = create_engine().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        assert!(
            try_load_cached(&engine, &wasm_bytes, &cwasm_path)
                .unwrap()
                .is_none(),
            "cache should be invalid when hash file is missing"
        );
    }

    #[test]
    fn cache_invalid_when_hash_file_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let hash_path = cwasm_path.with_extension("cwasm.sha256");
        std::fs::write(&hash_path, "somehash\n").unwrap();

        let engine = create_engine().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        assert!(
            try_load_cached(&engine, &wasm_bytes, &cwasm_path)
                .unwrap()
                .is_none(),
            "cache should be invalid with truncated hash file"
        );
    }

    #[test]
    fn compile_and_cache_rejects_invalid_wasm() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("garbage.wasm");
        std::fs::write(&wasm_path, b"this is not wasm").unwrap();

        let result = compile_and_cache(&wasm_path);
        assert!(result.is_err(), "should reject invalid WASM binary");
    }

    #[test]
    fn load_module_recompiles_on_corrupt_cache() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        // Corrupt the cached module but leave the hash file
        let cwasm_path = wasm_path.with_extension("cwasm");
        std::fs::write(&cwasm_path, b"corrupted cwasm data").unwrap();

        // load_module should detect corruption and fall back to source compilation
        let engine = create_engine().unwrap();
        let module = load_module(&engine, &wasm_path);
        assert!(
            module.is_ok(),
            "should recompile from source when cache is corrupt: {:?}",
            module.err()
        );
        assert!(module.unwrap().get_export("_start").is_some());
    }

    #[test]
    fn compile_and_cache_produces_all_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        compile_and_cache(&wasm_path).unwrap();

        let cwasm_path = wasm_path.with_extension("cwasm");
        let hash_path = cwasm_path.with_extension("cwasm.sha256");

        assert!(cwasm_path.exists(), ".cwasm should be created");
        assert!(hash_path.exists(), ".cwasm.sha256 should be created");
        assert!(
            std::fs::metadata(&cwasm_path).unwrap().len() > 0,
            ".cwasm should not be empty"
        );

        let stamp = std::fs::read_to_string(&hash_path).unwrap();
        let lines: Vec<&str> = stamp.lines().collect();
        assert_eq!(lines.len(), 3, "stamp should have 3 lines");
        assert_eq!(lines[0].len(), 64, "wasm hash should be 64 hex chars");
        assert_eq!(lines[1], WASMTIME_VERSION);
        assert_eq!(lines[2].len(), 64, "cwasm hash should be 64 hex chars");
    }

    // --- Capability denial for store / metadata ---

    #[test]
    fn capability_denial_store_traps() {
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (import "fledge" "store_set" (func $store_set (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "_start") (call $exit (i32.const 0)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("store_denied.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default(); // store = false
        let result = run_wasm_plugin(
            &wasm_path,
            &[],
            "test-store-denied",
            "0.0.1",
            dir.path(),
            &caps,
        );
        assert!(
            result.is_err(),
            "should fail when module imports store_set but store not granted"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("store"),
            "error should hint at missing store capability: {err}"
        );
    }

    #[test]
    fn capability_denial_metadata_traps() {
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (import "fledge" "metadata" (func $metadata (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (func (export "_start") (call $exit (i32.const 0)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("meta_denied.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default(); // metadata = false
        let result = run_wasm_plugin(
            &wasm_path,
            &[],
            "test-meta-denied",
            "0.0.1",
            dir.path(),
            &caps,
        );
        assert!(
            result.is_err(),
            "should fail when module imports metadata but capability not granted"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("metadata"),
            "error should hint at missing metadata capability: {err}"
        );
    }

    #[test]
    fn capability_denial_multiple_imports_shows_hint() {
        // Wasmtime reports the first unresolved import; verify we still produce a capability hint
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (import "fledge" "exec" (func $exec (param i32 i32) (result i32)))
            (import "fledge" "store_set" (func $store_set (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "_start") (call $exit (i32.const 0)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("multi_denied.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(
            &wasm_path,
            &[],
            "test-multi-denied",
            "0.0.1",
            dir.path(),
            &caps,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("capabilities not granted"),
            "error should mention missing capabilities: {err}"
        );
    }

    // --- Missing _start export ---

    #[test]
    fn missing_start_export_returns_error() {
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "main") (call $exit (i32.const 0)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("no_start.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-no-start", "0.0.1", dir.path(), &caps);
        assert!(result.is_err(), "should fail without _start export");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("_start"),
            "error should mention missing _start: {err}"
        );
    }

    // --- recv reads init context ---

    #[test]
    fn recv_delivers_init_context() {
        // Module that calls recv into a buffer, then exits 0 if it got data, 1 if not
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "_start")
                ;; recv into memory at offset 0, max 4096 bytes
                (if (i32.gt_s (call $recv (i32.const 0) (i32.const 4096)) (i32.const 0))
                    (then (call $exit (i32.const 0)))  ;; got data → success
                    (else (call $exit (i32.const 1)))  ;; no data → failure
                )
            )
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("recv_init.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(
            &wasm_path,
            &["--test-arg".to_string()],
            "test-recv-init",
            "0.0.1",
            dir.path(),
            &caps,
        );
        assert!(
            result.is_ok(),
            "plugin should receive init context via recv: {:?}",
            result.err()
        );
    }

    // --- Empty WASM file ---

    #[test]
    fn empty_wasm_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("empty.wasm");
        std::fs::write(&wasm_path, b"").unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-empty", "0.0.1", dir.path(), &caps);
        assert!(result.is_err(), "empty WASM file should fail to load");
    }

    // --- Exit codes ---

    #[test]
    fn exit_code_42_returns_error_with_code() {
        let wat = r#"(module
            (import "fledge" "exit" (func $exit (param i32)))
            (import "fledge" "recv" (func $recv (param i32 i32) (result i32)))
            (import "fledge" "send" (func $send (param i32 i32)))
            (memory (export "memory") 1)
            (func (export "_start") (call $exit (i32.const 42)))
        )"#;
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("exit42.wasm");
        std::fs::write(&wasm_path, wat).unwrap();

        let caps = PluginCapabilities::default();
        let result = run_wasm_plugin(&wasm_path, &[], "test-exit42", "0.0.1", dir.path(), &caps);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("42"),
            "error should include exit code 42: {err}"
        );
    }

    // --- FledgeExitOk display ---

    #[test]
    fn fledge_exit_ok_display() {
        let exit_ok = FledgeExitOk;
        assert_eq!(format!("{exit_ok}"), "plugin exited successfully");
    }

    // --- Capabilities enabled: run with exec/store/metadata granted ---

    #[test]
    fn run_with_all_capabilities_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("all_caps.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        let caps = PluginCapabilities {
            exec: true,
            store: true,
            metadata: true,
            filesystem: Some("plugin".to_string()),
            network: false,
        };
        let result = run_wasm_plugin(&wasm_path, &[], "test-all-caps", "0.0.1", dir.path(), &caps);
        assert!(
            result.is_ok(),
            "should succeed with all capabilities enabled: {:?}",
            result.err()
        );
    }

    #[test]
    fn filesystem_plugin_creates_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let wasm_path = dir.path().join("fs_plugin.wasm");
        std::fs::write(&wasm_path, EXIT_OK_WAT).unwrap();

        let caps = PluginCapabilities {
            filesystem: Some("plugin".to_string()),
            ..PluginCapabilities::default()
        };
        let result = run_wasm_plugin(
            &wasm_path,
            &[],
            "test-fs-plugin",
            "0.0.1",
            dir.path(),
            &caps,
        );
        assert!(result.is_ok(), "should succeed: {:?}", result.err());
        assert!(
            dir.path().join("data").exists(),
            "plugin filesystem mode should create data/ directory"
        );
    }

    // --- Filesystem grants: /project is read-only, /plugin is read-write ---
    //
    // These drive real WASI preview1 calls against the preopens build_wasi_p1
    // hands the guest, so a change in how the grants map onto wasmtime-wasi's
    // permissions (DirPerms/FilePerms before 49, FsPerms since) shows up as a
    // different errno or a file appearing on the host.

    const ERRNO_SUCCESS: i32 = 0;
    const ERRNO_BADF: i32 = 8;
    const ERRNO_PERM: i32 = 63;
    const OFLAGS_CREAT: i32 = 1;
    const OFLAGS_TRUNC: i32 = 8;
    const RIGHTS_FD_READ: i64 = 1 << 1;
    const RIGHTS_FD_WRITE: i64 = 1 << 6;
    // Preopens follow stdin, stdout and stderr, in the order build_wasi_p1
    // adds them: /project first, then /plugin.
    const FD_PROJECT: i32 = 3;
    const FD_PLUGIN: i32 = 4;

    fn path_open_probe(fd: i32, path: &str, oflags: i32, rights: i64) -> String {
        format!(
            r#"(module
                (import "wasi_snapshot_preview1" "path_open"
                    (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 16) "{path}")
                (func (export "probe") (result i32)
                    (call $path_open
                        (i32.const {fd}) (i32.const 0)
                        (i32.const 16) (i32.const {len})
                        (i32.const {oflags}) (i64.const {rights}) (i64.const 0)
                        (i32.const 0) (i32.const 0))))"#,
            len = path.len()
        )
    }

    fn create_dir_probe(fd: i32, path: &str) -> String {
        format!(
            r#"(module
                (import "wasi_snapshot_preview1" "path_create_directory"
                    (func $mkdir (param i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 16) "{path}")
                (func (export "probe") (result i32)
                    (call $mkdir (i32.const {fd}) (i32.const 16) (i32.const {len}))))"#,
            len = path.len()
        )
    }

    /// Runs the module's `probe` export in the sandbox `run_wasm_plugin` builds
    /// for `caps`, with `project` as the project root, and returns the errno.
    fn run_fs_probe(
        wat: &str,
        caps: &PluginCapabilities,
        plugin_dir: &Path,
        project: &Path,
    ) -> i32 {
        let engine = create_engine().unwrap();
        let module = Module::new(&engine, wat).unwrap();
        let wasi = build_wasi_p1(caps, plugin_dir, Some(project)).unwrap();
        let host_state = HostState {
            wasi,
            plugin_name: "test-fs-probe".to_string(),
            plugin_dir: plugin_dir.to_path_buf(),
            capabilities: caps.clone(),
            pending_response: None,
            limits: StoreLimitsBuilder::new()
                .memory_size(MAX_MEMORY_BYTES)
                .build(),
        };
        let mut store = Store::new(&engine, host_state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(FUEL_LIMIT).unwrap();
        store.set_epoch_deadline(1);
        let linker = setup_linker(&engine, caps).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let probe = instance
            .get_typed_func::<(), i32>(&mut store, "probe")
            .unwrap();
        probe.call(&mut store, ()).unwrap()
    }

    fn fs_caps(scope: &str) -> PluginCapabilities {
        PluginCapabilities {
            filesystem: Some(scope.to_string()),
            ..PluginCapabilities::default()
        }
    }

    #[test]
    fn project_scope_can_read_project_files() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("readme.txt"), "hello").unwrap();

        let wat = path_open_probe(FD_PROJECT, "readme.txt", 0, RIGHTS_FD_READ);
        let errno = run_fs_probe(&wat, &fs_caps("project"), plugin.path(), project.path());
        assert_eq!(
            errno, ERRNO_SUCCESS,
            "reading under /project must be allowed"
        );
    }

    #[test]
    fn project_scope_cannot_create_files_in_project() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        let wat = path_open_probe(FD_PROJECT, "new.txt", OFLAGS_CREAT, RIGHTS_FD_WRITE);
        let errno = run_fs_probe(&wat, &fs_caps("project"), plugin.path(), project.path());
        assert_eq!(errno, ERRNO_PERM, "creating under /project must be refused");
        assert!(!project.path().join("new.txt").exists());
    }

    #[test]
    fn project_scope_cannot_write_or_truncate_project_files() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let readme = project.path().join("readme.txt");
        std::fs::write(&readme, "hello").unwrap();

        for oflags in [0, OFLAGS_TRUNC] {
            let wat = path_open_probe(FD_PROJECT, "readme.txt", oflags, RIGHTS_FD_WRITE);
            let errno = run_fs_probe(&wat, &fs_caps("project"), plugin.path(), project.path());
            assert_eq!(
                errno, ERRNO_PERM,
                "write-open (oflags {oflags}) must be refused"
            );
        }
        assert_eq!(std::fs::read_to_string(&readme).unwrap(), "hello");
    }

    #[test]
    fn project_scope_cannot_create_directories_in_project() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        let wat = create_dir_probe(FD_PROJECT, "sub");
        let errno = run_fs_probe(&wat, &fs_caps("project"), plugin.path(), project.path());
        assert_eq!(errno, ERRNO_PERM, "mkdir under /project must be refused");
        assert!(!project.path().join("sub").exists());
    }

    #[test]
    fn plugin_scope_can_write_plugin_data_dir() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let caps = fs_caps("plugin");

        let wat = path_open_probe(FD_PLUGIN, "new.txt", OFLAGS_CREAT, RIGHTS_FD_WRITE);
        let errno = run_fs_probe(&wat, &caps, plugin.path(), project.path());
        assert_eq!(
            errno, ERRNO_SUCCESS,
            "creating under /plugin must be allowed"
        );
        assert!(plugin.path().join("data").join("new.txt").is_file());

        let wat = create_dir_probe(FD_PLUGIN, "sub");
        let errno = run_fs_probe(&wat, &caps, plugin.path(), project.path());
        assert_eq!(errno, ERRNO_SUCCESS, "mkdir under /plugin must be allowed");
        assert!(plugin.path().join("data").join("sub").is_dir());
    }

    #[test]
    fn plugin_scope_keeps_project_read_only() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        let wat = path_open_probe(FD_PROJECT, "new.txt", OFLAGS_CREAT, RIGHTS_FD_WRITE);
        let errno = run_fs_probe(&wat, &fs_caps("plugin"), plugin.path(), project.path());
        assert_eq!(
            errno, ERRNO_PERM,
            "/project stays read-only under the plugin scope"
        );
        assert!(!project.path().join("new.txt").exists());
    }

    #[test]
    fn plugin_scope_cannot_escape_data_dir() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        let wat = path_open_probe(FD_PLUGIN, "../escape.txt", OFLAGS_CREAT, RIGHTS_FD_WRITE);
        let errno = run_fs_probe(&wat, &fs_caps("plugin"), plugin.path(), project.path());
        assert_ne!(errno, ERRNO_SUCCESS, "`..` must not leave /plugin");
        assert!(!plugin.path().join("escape.txt").exists());
    }

    #[test]
    fn no_filesystem_grant_has_no_preopens() {
        let plugin = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("readme.txt"), "hello").unwrap();

        let wat = path_open_probe(FD_PROJECT, "readme.txt", 0, RIGHTS_FD_READ);
        let caps = PluginCapabilities::default();
        let errno = run_fs_probe(&wat, &caps, plugin.path(), project.path());
        assert_eq!(
            errno, ERRNO_BADF,
            "without a filesystem grant there is no fd 3"
        );
    }
}
