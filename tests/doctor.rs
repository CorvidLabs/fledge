mod common;
use common::*;

// Doctor command
// ──────────────────────────────────────────────────────────

#[test]
fn cli_doctor_succeeds() {
    let output = run_fledge(&["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") || stdout.contains("Git"));
}

#[test]
fn cli_doctor_json_valid() {
    let output = run_fledge(&["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["sections"].is_array());
    assert!(parsed["passed"].is_number());
    assert!(parsed["failed"].is_number());
}

// ──────────────────────────────────────────────────────────
