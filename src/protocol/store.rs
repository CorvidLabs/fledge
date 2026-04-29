use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(crate) const MAX_STORE_KEY_SIZE: usize = 256;
pub(crate) const MAX_STORE_VALUE_SIZE: usize = 64 * 1024; // 64 KB per value
pub(crate) const MAX_STORE_TOTAL_SIZE: usize = 1024 * 1024; // 1 MB total
pub(crate) const MAX_STORE_KEY_COUNT: usize = 256;

pub(crate) fn handle_store(plugin_dir: &Path, key: &str, value: &str) -> Result<()> {
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

pub(crate) fn handle_load(plugin_dir: &Path, key: &str) -> Result<serde_json::Value> {
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
