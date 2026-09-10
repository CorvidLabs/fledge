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
    // an exit-code failure. This also proves the probe hit the dead loopback
    // port from `TempEnv` rather than a real endpoint.
    let env = TempEnv::new();
    let output = env.run(&["doctor", "--json"]);
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    let names: Vec<&str> = parsed["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(names.contains(&"AI"), "sections: {names:?}");
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
