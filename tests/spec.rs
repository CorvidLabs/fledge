mod common;
use common::*;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// MARK: - spec commands
// Spec commands
// ──────────────────────────────────────────────────────────

#[test]
fn cli_spec_check_succeeds_in_project() {
    let output = run_fledge(&["spec", "check"]);
    // This runs against the fledge project itself which has specs
    assert!(output.status.success());
}

#[test]
fn cli_spec_init_in_new_dir() {
    let tmp = TempDir::new().unwrap();
    // init git repo first since spec needs project root
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let output = run_fledge_in(tmp.path(), &["spec", "init"]);
    assert!(output.status.success());
    assert!(tmp.path().join("specs").exists());
}

#[test]
fn cli_spec_new_creates_spec() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    // Init spec-sync first
    run_fledge_in(tmp.path(), &["spec", "init"]);

    let output = run_fledge_in(tmp.path(), &["spec", "new", "auth"]);
    assert!(output.status.success());
    assert!(tmp.path().join("specs/auth").exists());
}

#[test]
fn cli_spec_list_in_project() {
    let output = run_fledge(&["spec", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("spec(s) found"),
        "expected summary line in output: {stdout}"
    );
}

#[test]
fn cli_spec_list_ls_alias() {
    let output = run_fledge(&["spec", "ls"]);
    assert!(output.status.success());
}

#[test]
fn cli_spec_list_json_valid() {
    let output = run_fledge(&["spec", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Tier-D envelope: {schema_version: 1, action: "spec_list", specs: [...]}
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_list"));
    let specs = parsed["specs"].as_array().expect("specs array");
    assert!(!specs.is_empty(), "fledge project should have specs");
    let first = &specs[0];
    for field in [
        "name",
        "version",
        "status",
        "path",
        "files",
        "section_count",
        "required_sections",
        "companions",
        "missing_companions",
    ] {
        assert!(first.get(field).is_some(), "missing field: {field}");
    }
}

#[test]
fn cli_spec_list_json_empty_dir() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    run_fledge_in(tmp.path(), &["spec", "init"]);

    let output = run_fledge_in(tmp.path(), &["spec", "list", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert!(parsed["specs"].as_array().unwrap().is_empty());
}

#[test]
fn cli_spec_show_existing_module() {
    let output = run_fledge(&["spec", "show", "spec"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("spec"));
    assert!(stdout.contains("sections"));
}

#[test]
fn cli_spec_show_json_valid() {
    let output = run_fledge(&["spec", "show", "spec", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Tier-D envelope: {schema_version: 1, action: "spec_show", spec: {...}}
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_show"));
    let spec = &parsed["spec"];
    assert!(spec.is_object());
    assert_eq!(spec["name"].as_str(), Some("spec"));
    assert!(spec["sections"].is_array());
    assert!(spec["companions"].is_array());
    assert!(spec["missing_companions"].is_array());
}

#[test]
fn cli_spec_show_missing_module_fails() {
    let output = run_fledge(&["spec", "show", "definitely-not-a-real-spec-xyz"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No spec found") || stderr.contains("not"));
}

#[test]
fn cli_spec_check_json_valid() {
    let output = run_fledge(&["spec", "check", "--json"]);
    // May pass or fail on the repo's specs; either way stdout must be JSON
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Tier-D envelope
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_check"));
    assert!(parsed["specs"].is_array());
    assert!(parsed["totals"].is_object());
    assert!(parsed["totals"]["checked"].is_number());
    assert!(parsed["totals"]["errors"].is_number());
    assert!(parsed["totals"]["warnings"].is_number());
    assert!(parsed["strict"].is_boolean());
}

#[test]
fn cli_spec_check_json_spec_shape() {
    let output = run_fledge(&["spec", "check", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let specs = parsed["specs"].as_array().unwrap();
    assert!(!specs.is_empty(), "fledge repo should have specs");
    let first = &specs[0];
    for field in [
        "name",
        "version",
        "status",
        "file_count",
        "section_count",
        "required_count",
        "errors",
        "warnings",
    ] {
        assert!(first.get(field).is_some(), "missing field: {field}");
    }
    assert!(first["errors"].is_array());
    assert!(first["warnings"].is_array());
}

#[test]
fn cli_work_start_help_shows_json_flag() {
    let output = run_fledge(&["work", "start", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_commit_help_shows_json_flag() {
    let output = run_fledge(&["work", "commit", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_push_help_shows_json_flag() {
    let output = run_fledge(&["work", "push", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_status_help_shows_json_flag() {
    let output = run_fledge(&["work", "status", "--help"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
}

#[test]
fn cli_work_status_json_in_repo() {
    // Run inside a temp git repo with a real branch — avoids the detached-HEAD
    // situation that CI check-out sometimes produces.
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = run_fledge_in(tmp.path(), &["work", "status", "--json"]);
    assert!(
        output.status.success(),
        "work status --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["branch"].as_str(), Some("feature"));
    assert_eq!(parsed["default"].as_str(), Some("main"));
    assert!(parsed["ahead"].is_number());
    // behind is either a number or null (base-not-fetched sentinel)
    assert!(parsed["behind"].is_number() || parsed["behind"].is_null());
    // dirty is a count of uncommitted files
    assert!(parsed["dirty"].is_number());
}

// ──────────────────────────────────────────────────────────

// MARK: - spec edge cases
// Spec edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_spec_check_in_empty_dir_fails() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let output = run_fledge_in(tmp.path(), &["spec", "check"]);
    // No specs dir — should fail or warn
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Either exits nonzero or prints a message about missing specs
    assert!(
        !output.status.success()
            || stdout.contains("No specs")
            || stderr.contains("No specs")
            || stdout.contains("specs"),
        "expected some feedback about missing specs, got stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn cli_spec_new_duplicate_name() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    run_fledge_in(tmp.path(), &["spec", "init"]);
    run_fledge_in(tmp.path(), &["spec", "new", "auth"]);

    // Second creation should fail or warn
    let output = run_fledge_in(tmp.path(), &["spec", "new", "auth"]);
    assert!(
        !output.status.success() || {
            let stderr = String::from_utf8(output.stderr.clone()).unwrap();
            let stdout = String::from_utf8(output.stdout.clone()).unwrap();
            stderr.contains("exists") || stdout.contains("exists")
        },
        "expected duplicate spec warning"
    );
}

// ──────────────────────────────────────────────────────────
// Changelog edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_changelog_nonexistent_tag_fails() {
    let output = run_fledge(&["changelog", "--tag", "v999.999.999"]);
    // Should fail or return empty — shouldn't panic
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("not found") || stderr.contains("999"),
            "expected tag-not-found error, got: {stderr}"
        );
    }
}

#[test]
fn cli_changelog_zero_limit() {
    let output = run_fledge(&["changelog", "--limit", "0"]);
    // Should succeed with empty output or handle gracefully
    assert!(output.status.success());
}

#[test]
fn cli_changelog_in_non_git_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["changelog"]);
    // Not a git repo — should fail gracefully
    assert!(
        !output.status.success() || {
            let stdout = String::from_utf8(output.stdout.clone()).unwrap();
            stdout.contains("No tags") || stdout.is_empty()
        }
    );
}

// ──────────────────────────────────────────────────────────
// Doctor edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_doctor_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fledge") || stdout.contains("Git"));
}

#[test]
fn cli_doctor_json_in_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let output = run_fledge_in(tmp.path(), &["doctor", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["sections"].is_array());
}

// ──────────────────────────────────────────────────────────
// Validate-template edge cases
// ──────────────────────────────────────────────────────────

#[test]
fn cli_validate_template_nonexistent_path() {
    let output = run_fledge(&["templates", "validate", "/tmp/no-such-path-ever-12345"]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_empty_template_toml() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("empty-tpl");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(tpl.join("template.toml"), "").unwrap();
    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_missing_name_field() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("noname");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
description = "Missing name field"

[files]
ignore = ["template.toml"]
"#,
    )
    .unwrap();
    fs::write(tpl.join("file.txt"), "content").unwrap();
    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    assert!(!output.status.success());
}

#[test]
fn cli_validate_template_missing_description() {
    let tmp = TempDir::new().unwrap();
    let tpl = tmp.path().join("nodesc");
    fs::create_dir_all(&tpl).unwrap();
    fs::write(
        tpl.join("template.toml"),
        r#"[template]
name = "nodesc"

[files]
ignore = ["template.toml"]
"#,
    )
    .unwrap();
    fs::write(tpl.join("file.txt"), "content").unwrap();
    let output = run_fledge(&["templates", "validate", tpl.to_str().unwrap()]);
    // Missing description might be a warning or error
    let _status = output.status;
}

// MARK: - spec lint
// ──────────────────────────────────────────────────────────

#[test]
fn cli_spec_lint_succeeds_in_project() {
    // fledge's own specs must clear the structural gate.
    let output = run_fledge(&["spec", "lint"]);
    assert!(
        output.status.success(),
        "spec lint failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_spec_lint_json_envelope() {
    let output = run_fledge(&["spec", "lint", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["schema_version"].as_u64(), Some(1));
    assert_eq!(parsed["action"].as_str(), Some("spec_lint"));
    assert_eq!(parsed["passed"].as_bool(), Some(true));
    // Layer 2 is opt-in, so a plain CI invocation never touches a provider.
    assert_eq!(parsed["model_pass"]["requested"].as_bool(), Some(false));
    assert_eq!(parsed["model_pass"]["ran"].as_bool(), Some(false));
    assert!(parsed["model_pass"]["skipped_reason"].is_string());
    let specs = parsed["specs"].as_array().expect("specs array");
    assert!(!specs.is_empty());
    for field in ["name", "path", "version", "status", "findings", "passed"] {
        assert!(specs[0].get(field).is_some(), "missing field {field}");
    }
    assert!(parsed["totals"]["linted"].as_u64().unwrap() > 0);
}

#[test]
fn cli_spec_lint_single_module() {
    let output = run_fledge(&["spec", "lint", "spec", "--json"]);
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(parsed["totals"]["linted"].as_u64(), Some(1));
    assert_eq!(parsed["specs"][0]["name"].as_str(), Some("spec"));
}

#[test]
fn cli_spec_lint_rejects_a_freshly_scaffolded_spec() {
    // The point of the command: `spec new` output is structurally complete but
    // says nothing, and `spec check` passes it. Lint must not.
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    run_fledge_in(tmp.path(), &["spec", "init"]);
    run_fledge_in(tmp.path(), &["spec", "new", "auth"]);

    let output = run_fledge_in(tmp.path(), &["spec", "lint", "--json"]);
    assert!(!output.status.success(), "scaffolded spec should fail lint");
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(parsed["passed"].as_bool(), Some(false));
    let checks: Vec<String> = parsed["specs"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["check"].as_str().unwrap().to_string())
        .collect();
    assert!(checks.contains(&"empty_section".to_string()), "{checks:?}");
    assert!(checks.contains(&"missing_file".to_string()), "{checks:?}");
    assert!(
        checks.contains(&"no_rejection_signal".to_string()),
        "{checks:?}"
    );
    // Errors go to stderr as plain text even under --json.
    assert!(String::from_utf8_lossy(&output.stderr).contains("spec lint failed"));
}

#[test]
fn cli_spec_lint_ignore_suppresses_a_check() {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    run_fledge_in(tmp.path(), &["spec", "init"]);
    run_fledge_in(tmp.path(), &["spec", "new", "auth"]);

    let output = run_fledge_in(
        tmp.path(),
        &["spec", "lint", "--ignore", "missing_file", "--json"],
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    let checks: Vec<String> = parsed["specs"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["check"].as_str().unwrap().to_string())
        .collect();
    assert!(!checks.contains(&"missing_file".to_string()), "{checks:?}");
    assert!(parsed["totals"]["ignored"].as_u64().unwrap() > 0);
}

/// A project whose `.specsync/config.toml` renames every default section.
/// Returns the tempdir with `.specsync/config.toml`, `src/demo.rs`, and
/// `specs/demo/demo.spec.md` written from `body`.
fn custom_sections_project(required: &str, body: &str) -> TempDir {
    let tmp = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    fs::create_dir_all(tmp.path().join(".specsync")).unwrap();
    fs::write(
        tmp.path().join(".specsync/config.toml"),
        format!("specs_dir = \"specs\"\nrequired_sections = {required}\n"),
    )
    .unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/demo.rs"), "// demo\n").unwrap();
    fs::create_dir_all(tmp.path().join("specs/demo")).unwrap();
    fs::write(tmp.path().join("specs/demo/demo.spec.md"), body).unwrap();
    tmp
}

const CUSTOM_REQUIRED: &str =
    r#"["Overview", "API", "Rules", "Examples", "Failures", "Deps", "History"]"#;

/// A complete spec under `CUSTOM_REQUIRED`'s headings.
fn custom_sections_spec(overview: &str) -> String {
    format!(
        r#"---
module: demo
version: 3
status: active
files:
  - src/demo.rs
---

# Demo

## Overview

{overview}

## API

| Export | Description |
|--------|-------------|
| `run` | Executes the validated plan |

## Rules

1. `run` never mutates the config it was given.

## Examples

Given a config with one task, when `run` is called, then the task executes once.

## Failures

| Error | When | Behavior |
|-------|------|----------|
| UnknownTask | the named task is absent | exits 1 and names the task |

## Deps

- serde

## History

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-01-01 | Initial spec |
"#
    )
}

#[test]
fn cli_spec_lint_honors_custom_required_sections() {
    // End-to-end for the two halves of the same bug. A project that renamed its
    // sections used to get the worst of both: placeholders in `## Overview` were
    // never seen (the check scanned a hardcoded "Purpose"), while full
    // `## Examples` / `## Failures` sections still failed the signal checks
    // (which scanned a hardcoded "Behavioral Examples" / "Error Cases").

    // Half one — a healthy custom-section spec passes cleanly.
    let healthy = custom_sections_project(
        CUSTOM_REQUIRED,
        &custom_sections_spec(
            "Demo turns a parsed config into a validated execution plan, so a\nmalformed config fails before any task runs.",
        ),
    );
    let output = run_fledge_in(healthy.path(), &["spec", "lint", "--json"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(
        parsed["passed"].as_bool(),
        Some(true),
        "custom sections should pass: {parsed}"
    );
    assert_eq!(parsed["totals"]["errors"].as_u64(), Some(0));
    assert_eq!(parsed["totals"]["warnings"].as_u64(), Some(0));

    // Half two — a placeholder in the renamed Purpose section is caught.
    let placeholder = custom_sections_project(
        CUSTOM_REQUIRED,
        &custom_sections_spec("TODO: fill this in later."),
    );
    let output = run_fledge_in(placeholder.path(), &["spec", "lint", "--json"]);
    assert!(!output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    let findings = parsed["specs"][0]["findings"].as_array().unwrap();
    let placeholder_finding = findings
        .iter()
        .find(|f| f["check"] == "placeholder_text")
        .unwrap_or_else(|| panic!("no placeholder_text finding in {parsed}"));
    assert_eq!(placeholder_finding["section"].as_str(), Some("Overview"));
}

#[test]
fn cli_spec_lint_warns_instead_of_failing_when_no_section_carries_a_signal() {
    // A `required_sections` with no acceptance/rejection analogue at all. The
    // checks degrade to warnings naming the `accepts:`/`rejects:` escape hatch
    // rather than becoming a red nothing in the project can clear.
    let spec = r#"---
module: demo
version: 3
status: active
files:
  - src/demo.rs
---

# Demo

## Overview

Demo turns a parsed config into a validated execution plan.

## Notes

Nothing here names an acceptance or rejection signal.
"#;
    let tmp = custom_sections_project(r#"["Overview", "Notes"]"#, spec);
    let output = run_fledge_in(tmp.path(), &["spec", "lint", "--json"]);
    assert!(
        output.status.success(),
        "unresolvable signals must not fail the gate: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(parsed["passed"].as_bool(), Some(true));
    assert_eq!(parsed["totals"]["errors"].as_u64(), Some(0));
    assert_eq!(parsed["totals"]["warnings"].as_u64(), Some(2));
    for finding in parsed["specs"][0]["findings"].as_array().unwrap() {
        assert_eq!(finding["severity"].as_str(), Some("warning"), "{finding}");
        assert!(finding["message"]
            .as_str()
            .unwrap()
            .contains("required_sections"));
    }
}

#[test]
fn cli_spec_lint_reports_an_overridden_ai_request_faithfully() {
    // `--no-ai` wins, but the JSON must still show that `--ai` was asked for:
    // `requested` is what the caller wanted, `ran` is what happened.
    let output = run_fledge(&["spec", "lint", "--ai", "--no-ai", "--json"]);
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    let model_pass = &parsed["model_pass"];
    assert_eq!(model_pass["requested"].as_bool(), Some(true));
    assert_eq!(model_pass["ran"].as_bool(), Some(false));
    assert_eq!(
        model_pass["skipped_reason"].as_str(),
        Some("--no-ai overrides --ai")
    );
    // No provider was ever built, so the run stayed offline.
    assert!(model_pass["provider"].is_null());
}

#[test]
fn cli_spec_lint_rejects_unknown_ignore_id() {
    let output = run_fledge(&["spec", "lint", "--ignore", "not_a_check"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown --ignore check"));
}

// ──────────────────────────────────────────────────────────
