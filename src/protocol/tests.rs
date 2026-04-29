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
    let json =
        r#"{"type":"multi_select","id":"4","message":"Pick:","options":["x","y"],"defaults":[0]}"#;
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

fn compile_test_plugin(src: &str, tmp: &std::path::Path) -> std::path::PathBuf {
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
    let result =
        super::run_protocol_plugin(&bin, &[], "test-no-store", "0.1.0", store_dir.path(), &caps);
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

#[test]
fn exec_output_cap_is_10mb() {
    assert_eq!(MAX_EXEC_OUTPUT_SIZE, 10 * 1024 * 1024);
}

#[test]
fn bounded_read_truncates_at_limit() {
    use std::io::Cursor;
    let data = vec![0xABu8; MAX_EXEC_OUTPUT_SIZE + 1024];
    let reader = Cursor::new(data);
    let mut buf = Vec::new();
    reader
        .take(MAX_EXEC_OUTPUT_SIZE as u64)
        .read_to_end(&mut buf)
        .unwrap();
    assert_eq!(buf.len(), MAX_EXEC_OUTPUT_SIZE);
}
