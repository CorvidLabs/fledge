mod common;
use common::*;

// Doctor command
// ──────────────────────────────────────────────────────────
//
// `doctor` reads the fledge config and probes the configured AI host, so every
// test here runs inside a `TempEnv`: isolated HOME / FLEDGE_CONFIG_DIR and an
// `OLLAMA_HOST` pointing at a closed loopback port. Without that isolation
// these tests would read the developer's real config and issue a request to
// whatever endpoint it names (issue #447).

#[test]
fn cli_doctor_succeeds() {
    let env = TempEnv::new();
    let output = env.run(&["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") || stdout.contains("Git"));
}

#[test]
fn cli_doctor_json_valid() {
    let env = TempEnv::new();
    let output = env.run(&["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("doctor"));
    assert!(parsed["sections"].is_array());
    assert!(parsed["passed"].is_number());
    assert!(parsed["failed"].is_number());
}

#[test]
fn cli_doctor_reports_unreachable_ai_host_without_failing() {
    // The AI section is diagnostic: an unreachable provider is reported, not
    // an exit-code failure.
    //
    // The detail string is asserted, not just the section name, because that
    // is the only part that names the host actually probed. With every
    // provider key stripped, `TempEnv` leaves ollama as the resolved provider
    // and points `OLLAMA_HOST` at a closed loopback port, so a passing
    // assertion here is evidence the probe stayed on this machine. Asserting
    // only that a section called "AI" exists holds no matter which endpoint
    // was contacted, which is what this test used to do.
    let env = TempEnv::new();
    let output = env.run(&["doctor", "--json"]);
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();

    let ai = parsed["sections"]
        .as_array()
        .expect("sections array")
        .iter()
        .find(|s| s["name"] == "AI")
        .unwrap_or_else(|| panic!("no AI section in {parsed}"));

    let provider = ai["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .find(|c| {
            c["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("Active provider:"))
        })
        .unwrap_or_else(|| panic!("no active-provider check in {ai}"));

    // Diagnostic, not fatal: the check is not ok, yet the command succeeded.
    assert_ne!(provider["status"], "ok", "provider check: {provider}");

    let detail = provider["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains("127.0.0.1"),
        "the probe must have targeted TempEnv's dead loopback port, got: {detail}"
    );
}

#[test]
fn cli_doctor_does_not_touch_the_real_config_dir() {
    // doctor is read-only with respect to config: nothing is written into the
    // isolated config dir, and the run still succeeds with no config present.
    let env = TempEnv::new();
    let output = env.run(&["doctor", "--json"]);
    assert!(output.status.success());
    assert!(
        !env.config_dir().join("config.toml").exists(),
        "doctor must not create a config file"
    );
}

// ──────────────────────────────────────────────────────────
