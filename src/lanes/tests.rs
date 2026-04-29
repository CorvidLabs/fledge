use std::collections::BTreeMap;

use crate::trust::determine_trust_tier;

use super::{
    base64_decode, create_lane_repo, execute_lane, format_duration, format_lane_toml,
    lane_defaults, list_lanes, load_lane_config, parse_import_source, validate_lane,
    validate_lanes, FledgeFileWithLanes, LaneDef, ParallelItem, Step,
};

fn parse_config(toml_str: &str) -> FledgeFileWithLanes {
    toml::from_str(toml_str).unwrap()
}

#[test]
fn parse_sequential_lane() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[lanes.ci]
description = "CI pipeline"
steps = ["lint", "test", "build"]
"#,
    );
    assert_eq!(config.lanes.len(), 1);
    assert_eq!(config.lanes["ci"].steps.len(), 3);
    assert!(config.lanes["ci"].fail_fast);
}

#[test]
fn parse_inline_step() {
    let config = parse_config(
        r#"
[tasks]
test = "cargo test"

[lanes.release]
description = "Release"
steps = [
  "test",
  { run = "cargo build --release" },
]
"#,
    );
    assert_eq!(config.lanes["release"].steps.len(), 2);
    match &config.lanes["release"].steps[1] {
        Step::Inline { run: cmd } => assert_eq!(cmd, "cargo build --release"),
        _ => panic!("expected inline step"),
    }
}

#[test]
fn parse_parallel_step() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"
fmt = "cargo fmt --check"
test = "cargo test"

[lanes.check]
description = "Quick check"
steps = [
  { parallel = ["lint", "fmt"] },
  "test"
]
"#,
    );
    assert_eq!(config.lanes["check"].steps.len(), 2);
    match &config.lanes["check"].steps[0] {
        Step::Parallel { parallel } => {
            assert_eq!(parallel.len(), 2);
            assert!(matches!(&parallel[0], ParallelItem::TaskRef(n) if n == "lint"));
            assert!(matches!(&parallel[1], ParallelItem::TaskRef(n) if n == "fmt"));
        }
        _ => panic!("expected parallel step"),
    }
}

#[test]
fn parse_fail_fast_false() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"
b = "echo b"

[lanes.audit]
description = "Audit"
fail_fast = false
steps = ["a", "b"]
"#,
    );
    assert!(!config.lanes["audit"].fail_fast);
}

#[test]
fn parse_fail_fast_default_true() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"

[lanes.ci]
steps = ["a"]
"#,
    );
    assert!(config.lanes["ci"].fail_fast);
}

#[test]
fn validate_unknown_task_ref() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
steps = ["lint", "nonexistent"]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nonexistent"));
}

#[test]
fn validate_unknown_parallel_ref() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"

[lanes.check]
steps = [{ parallel = ["lint", "ghost"] }]
"#,
    );
    let result = validate_lane("check", &config.lanes["check"], &config.tasks);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("ghost"));
}

#[test]
fn validate_inline_always_ok() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo hello" }]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_ok());
}

#[test]
fn validate_all_valid_refs() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[lanes.ci]
steps = ["lint", "test", "build"]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_ok());
}

#[test]
fn validate_circular_deps() {
    let config = parse_config(
        r#"
[tasks.a]
cmd = "echo a"
deps = ["b"]

[tasks.b]
cmd = "echo b"
deps = ["a"]

[lanes.ci]
steps = ["a"]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("circular"),
        "expected circular error, got: {err}"
    );
}

#[test]
fn validate_no_cycle_with_shared_deps() {
    let config = parse_config(
        r#"
[tasks]
common = "echo common"

[tasks.a]
cmd = "echo a"
deps = ["common"]

[tasks.b]
cmd = "echo b"
deps = ["common"]

[lanes.ci]
steps = ["a", "b"]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_ok());
}

#[test]
fn parse_multiple_lanes() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"
build = "cargo build"

[lanes.ci]
description = "CI"
steps = ["lint", "test", "build"]

[lanes.quick]
description = "Quick"
steps = ["lint"]
"#,
    );
    assert_eq!(config.lanes.len(), 2);
    assert!(config.lanes.contains_key("ci"));
    assert!(config.lanes.contains_key("quick"));
}

#[test]
fn parse_no_lanes_section() {
    let config = parse_config(
        r#"
[tasks]
build = "cargo build"
"#,
    );
    assert!(config.lanes.is_empty());
}

#[test]
fn parse_empty_lanes_section() {
    let config = parse_config(
        r#"
[tasks]
build = "cargo build"

[lanes]
"#,
    );
    assert!(config.lanes.is_empty());
}

#[test]
fn parse_mixed_step_types() {
    let config = parse_config(
        r#"
[tasks]
test = "cargo test"
lint = "cargo clippy"

[lanes.full]
steps = [
  "test",
  { run = "echo done" },
  { parallel = ["test", "lint"] },
]
"#,
    );
    assert_eq!(config.lanes["full"].steps.len(), 3);
    assert!(matches!(&config.lanes["full"].steps[0], Step::TaskRef(_)));
    assert!(matches!(
        &config.lanes["full"].steps[1],
        Step::Inline { .. }
    ));
    assert!(matches!(
        &config.lanes["full"].steps[2],
        Step::Parallel { .. }
    ));
}

#[test]
fn execute_sequential_lane_echo() {
    let config = parse_config(
        r#"
[tasks]
a = "echo step-a"
b = "echo step-b"

[lanes.seq]
description = "Sequential"
steps = ["a", "b"]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "seq",
        &config.lanes["seq"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn execute_inline_step() {
    let config = parse_config(
        r#"
[tasks]

[lanes.inline]
steps = [{ run = "echo inline-works" }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "inline",
        &config.lanes["inline"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn execute_parallel_step() {
    let config = parse_config(
        r#"
[tasks]
a = "echo parallel-a"
b = "echo parallel-b"

[lanes.par]
steps = [{ parallel = ["a", "b"] }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "par",
        &config.lanes["par"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn parse_parallel_inline_items() {
    let config = parse_config(
        r#"
[tasks]
lint = "cargo clippy"

[lanes.mixed]
description = "Mixed parallel"
steps = [
  { parallel = ["lint", { run = "echo inline" }] },
]
"#,
    );
    match &config.lanes["mixed"].steps[0] {
        Step::Parallel { parallel } => {
            assert_eq!(parallel.len(), 2);
            assert!(matches!(&parallel[0], ParallelItem::TaskRef(n) if n == "lint"));
            assert!(matches!(&parallel[1], ParallelItem::Inline { run } if run == "echo inline"));
        }
        _ => panic!("expected parallel step"),
    }
}

#[test]
fn execute_parallel_with_inline() {
    let config = parse_config(
        r#"
[tasks]
a = "echo task-a"

[lanes.mixed]
steps = [{ parallel = ["a", { run = "echo inline-b" }] }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "mixed",
        &config.lanes["mixed"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn execute_parallel_all_inline() {
    let config = parse_config(
        r#"
[tasks]

[lanes.inlines]
steps = [{ parallel = [{ run = "echo one" }, { run = "echo two" }] }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "inlines",
        &config.lanes["inlines"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn validate_parallel_inline_no_task_check() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ parallel = [{ run = "echo hello" }] }]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_ok());
}

#[test]
fn format_lane_toml_with_parallel_inline() {
    let lane = LaneDef {
        description: None,
        steps: vec![Step::Parallel {
            parallel: vec![
                ParallelItem::TaskRef("lint".to_string()),
                ParallelItem::Inline {
                    run: "echo done".to_string(),
                },
            ],
        }],
        fail_fast: true,
        source: None,
    };
    let toml = format_lane_toml("mixed", &lane);
    assert!(toml.contains("\"lint\""));
    assert!(toml.contains("{ run = \"echo done\" }"));
}

#[test]
fn execute_fail_fast_stops() {
    let config = parse_config(
        r#"
[tasks]
fail = "exit 1"
ok = "echo ok"

[lanes.ff]
fail_fast = true
steps = ["fail", "ok"]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ff",
        &config.lanes["ff"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("failed at step 1"));
}

#[test]
fn execute_no_fail_fast_continues() {
    let config = parse_config(
        r#"
[tasks]
fail = "exit 1"
ok = "echo ok"

[lanes.noff]
fail_fast = false
steps = ["fail", "ok"]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "noff",
        &config.lanes["noff"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("1 failure"));
}

#[test]
fn execute_task_deps_in_lane() {
    let config = parse_config(
        r#"
[tasks.build]
cmd = "echo building"
deps = ["prep"]

[tasks.prep]
cmd = "echo preparing"

[lanes.ci]
steps = ["build"]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn lane_defaults_are_valid_toml() {
    for project_type in &["rust", "node", "go", "python", "generic"] {
        let tasks = match *project_type {
            "rust" => {
                "[tasks]\nfmt = \"cargo fmt\"\nlint = \"cargo clippy\"\ntest = \"cargo test\"\nbuild = \"cargo build\"\ntypecheck = \"echo ok\"\n"
            }
            "node" => {
                "[tasks]\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n"
            }
            "go" => {
                "[tasks]\nfmt = \"echo fmt\"\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n"
            }
            "python" => {
                "[tasks]\nfmt = \"echo fmt\"\nlint = \"echo lint\"\ntypecheck = \"echo tc\"\ntest = \"echo test\"\n"
            }
            _ => {
                "[tasks]\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n"
            }
        };
        let defaults = lane_defaults(project_type);
        let toml_str = format!("{}{}", tasks, defaults);
        let result: Result<FledgeFileWithLanes, _> = toml::from_str(&toml_str);
        assert!(
            result.is_ok(),
            "Invalid TOML for {}: {:?}",
            project_type,
            result.err()
        );
    }
}

#[test]
fn parse_import_source_basic() {
    let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes");
    assert_eq!(owner, "CorvidLabs");
    assert_eq!(repo, "fledge-lanes");
    assert!(subpath.is_none());
    assert!(git_ref.is_none());
}

#[test]
fn parse_import_source_with_ref() {
    let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes@v1.0.0");
    assert_eq!(owner, "CorvidLabs");
    assert_eq!(repo, "fledge-lanes");
    assert!(subpath.is_none());
    assert_eq!(git_ref.unwrap(), "v1.0.0");
}

#[test]
fn parse_import_source_with_subpath() {
    let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes/rust");
    assert_eq!(owner, "CorvidLabs");
    assert_eq!(repo, "fledge-lanes");
    assert_eq!(subpath.unwrap(), "rust");
    assert!(git_ref.is_none());
}

#[test]
fn parse_import_source_with_subpath_and_ref() {
    let (owner, repo, subpath, git_ref) = parse_import_source("CorvidLabs/fledge-lanes/rust@main");
    assert_eq!(owner, "CorvidLabs");
    assert_eq!(repo, "fledge-lanes");
    assert_eq!(subpath.unwrap(), "rust");
    assert_eq!(git_ref.unwrap(), "main");
}

#[test]
fn parse_import_source_full_url() {
    let (owner, repo, subpath, git_ref) =
        parse_import_source("https://github.com/CorvidLabs/fledge-lanes.git");
    assert_eq!(owner, "CorvidLabs");
    assert_eq!(repo, "fledge-lanes");
    assert!(subpath.is_none());
    assert!(git_ref.is_none());
}

#[test]
fn parse_import_source_url_with_ref() {
    let (owner, repo, subpath, git_ref) =
        parse_import_source("https://github.com/CorvidLabs/fledge-lanes@main");
    assert_eq!(owner, "CorvidLabs");
    assert_eq!(repo, "fledge-lanes");
    assert!(subpath.is_none());
    assert_eq!(git_ref.unwrap(), "main");
}

#[test]
fn format_lane_toml_sequential() {
    let lane = LaneDef {
        description: Some("CI pipeline".to_string()),
        steps: vec![
            Step::TaskRef("lint".to_string()),
            Step::TaskRef("test".to_string()),
        ],
        fail_fast: true,
        source: None,
    };
    let toml = format_lane_toml("ci", &lane);
    assert!(toml.contains("[lanes.ci]"));
    assert!(toml.contains("description = \"CI pipeline\""));
    assert!(toml.contains("\"lint\""));
    assert!(toml.contains("\"test\""));
    assert!(!toml.contains("fail_fast"));
}

#[test]
fn format_lane_toml_with_fail_fast_false() {
    let lane = LaneDef {
        description: None,
        steps: vec![Step::TaskRef("audit".to_string())],
        fail_fast: false,
        source: None,
    };
    let toml = format_lane_toml("audit", &lane);
    assert!(toml.contains("fail_fast = false"));
}

#[test]
fn format_lane_toml_with_inline() {
    let lane = LaneDef {
        description: None,
        steps: vec![Step::Inline {
            run: "echo hello".to_string(),
        }],
        fail_fast: true,
        source: None,
    };
    let toml = format_lane_toml("test", &lane);
    assert!(toml.contains("{ run = \"echo hello\" }"));
}

#[test]
fn format_lane_toml_with_parallel() {
    let lane = LaneDef {
        description: None,
        steps: vec![Step::Parallel {
            parallel: vec![
                ParallelItem::TaskRef("lint".to_string()),
                ParallelItem::TaskRef("fmt".to_string()),
            ],
        }],
        fail_fast: true,
        source: None,
    };
    let toml = format_lane_toml("check", &lane);
    assert!(toml.contains("parallel"));
    assert!(toml.contains("\"lint\""));
    assert!(toml.contains("\"fmt\""));
}

#[test]
fn format_lane_toml_roundtrips() {
    let lane = LaneDef {
        description: Some("Full CI".to_string()),
        steps: vec![
            Step::TaskRef("lint".to_string()),
            Step::TaskRef("test".to_string()),
            Step::TaskRef("build".to_string()),
        ],
        fail_fast: true,
        source: None,
    };
    let toml_str = format!(
        "[tasks]\nlint = \"echo lint\"\ntest = \"echo test\"\nbuild = \"echo build\"\n{}",
        format_lane_toml("ci", &lane)
    );
    let parsed: FledgeFileWithLanes = toml::from_str(&toml_str).unwrap();
    assert!(parsed.lanes.contains_key("ci"));
    assert_eq!(parsed.lanes["ci"].steps.len(), 3);
}

#[test]
fn base64_decode_basic() {
    let encoded = "SGVsbG8gV29ybGQ=";
    let decoded = base64_decode(encoded).unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "Hello World");
}

#[test]
fn base64_decode_no_padding() {
    let encoded = "Zm9v";
    let decoded = base64_decode(encoded).unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "foo");
}

#[test]
fn base64_decode_empty() {
    let decoded = base64_decode("").unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn base64_decode_with_newlines() {
    let encoded = "SGVs\nbG8=";
    let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64_decode(&cleaned).unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "Hello");
}

#[test]
fn format_duration_millis() {
    let d = std::time::Duration::from_millis(42);
    assert_eq!(format_duration(d), "42ms");
}

#[test]
fn format_duration_seconds() {
    let d = std::time::Duration::from_millis(3456);
    assert_eq!(format_duration(d), "3.456s");
}

#[test]
fn format_duration_minutes() {
    let d = std::time::Duration::from_secs(125) + std::time::Duration::from_millis(100);
    assert_eq!(format_duration(d), "2m 5.100s");
}

#[test]
fn format_duration_zero() {
    let d = std::time::Duration::from_millis(0);
    assert_eq!(format_duration(d), "0ms");
}

#[test]
fn merge_imported_lanes() {
    let mut base = parse_config(
        r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
steps = ["lint"]
"#,
    );
    let imported = parse_config(
        r#"
[tasks]
lint = "overridden"
test = "cargo test"

[lanes.ci]
steps = ["lint", "test"]

[lanes.deploy]
steps = ["test"]
"#,
    );

    for (name, task) in imported.tasks {
        base.tasks.entry(name).or_insert(task);
    }
    for (name, lane) in imported.lanes {
        base.lanes.entry(name).or_insert(lane);
    }

    assert_eq!(base.tasks["lint"].cmd(), "cargo clippy");
    assert_eq!(base.tasks["test"].cmd(), "cargo test");
    assert_eq!(base.lanes["ci"].steps.len(), 1);
    assert!(base.lanes.contains_key("deploy"));
}

#[test]
fn create_lane_repo_scaffolds_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    create_lane_repo("my-lanes", tmp.path(), Some("Test lanes"), true, false).unwrap();

    let target = tmp.path().join("my-lanes");
    assert!(target.join("fledge.toml").exists());
    assert!(target.join("README.md").exists());
    assert!(target.join(".gitignore").exists());

    let content = std::fs::read_to_string(target.join("fledge.toml")).unwrap();
    let parsed: FledgeFileWithLanes = toml::from_str(&content).unwrap();
    assert!(!parsed.lanes.is_empty());
    assert!(!parsed.tasks.is_empty());
}

#[test]
fn create_lane_repo_fails_if_exists() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join("existing")).unwrap();
    let result = create_lane_repo("existing", tmp.path(), None, true, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn validate_valid_lanes() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        r#"
[tasks]
lint = "cargo clippy"
test = "cargo test"

[lanes.ci]
description = "CI pipeline"
steps = ["lint", "test"]
"#,
    )
    .unwrap();

    let result = validate_lanes(tmp.path(), false, false);
    assert!(result.is_ok());
}

#[test]
fn validate_undefined_task_ref() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
description = "CI"
steps = ["lint", "nonexistent"]
"#,
    )
    .unwrap();

    let result = validate_lanes(tmp.path(), false, false);
    assert!(result.is_err());
}

#[test]
fn validate_empty_steps() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        r#"
[lanes.empty]
description = "Empty"
steps = []
"#,
    )
    .unwrap();

    let result = validate_lanes(tmp.path(), false, false);
    assert!(result.is_err());
}

#[test]
fn validate_missing_description_warns() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
steps = ["lint"]
"#,
    )
    .unwrap();

    // non-strict: passes with warnings
    let result = validate_lanes(tmp.path(), false, false);
    assert!(result.is_ok());

    // strict: fails on warnings
    let result = validate_lanes(tmp.path(), true, false);
    assert!(result.is_err());
}

#[test]
fn validate_no_lanes_is_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        r#"
[tasks]
lint = "cargo clippy"
"#,
    )
    .unwrap();

    let result = validate_lanes(tmp.path(), false, false);
    assert!(result.is_err());
}

#[test]
fn validate_json_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("fledge.toml"),
        r#"
[tasks]
lint = "cargo clippy"

[lanes.ci]
description = "CI"
steps = ["lint"]
"#,
    )
    .unwrap();

    let result = validate_lanes(tmp.path(), false, true);
    assert!(result.is_ok());
}

#[test]
fn imported_lanes_get_source_tracked() {
    let tmp = tempfile::TempDir::new().unwrap();
    let fledge_toml = tmp.path().join("fledge.toml");
    std::fs::write(
        &fledge_toml,
        "[tasks]\nlint = \"echo lint\"\n\n[lanes.local]\nsteps = [\"lint\"]\n",
    )
    .unwrap();

    let lanes_dir = tmp.path().join(".fledge").join("lanes");
    std::fs::create_dir_all(&lanes_dir).unwrap();
    std::fs::write(
        lanes_dir.join("corvidlabs-fledge-lanes.toml"),
        "# Imported from CorvidLabs/fledge-lanes\n\n[tasks]\ntest = \"echo test\"\n\n[lanes.ci]\ndescription = \"CI\"\nsteps = [\"lint\", \"test\"]\n",
    )
    .unwrap();
    std::fs::write(
        lanes_dir.join("someuser-lanes.toml"),
        "# Imported from someuser/lanes\n\n[lanes.deploy]\nsteps = [{ run = \"echo deploy\" }]\n",
    )
    .unwrap();

    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();
    let config = load_lane_config().unwrap();
    std::env::set_current_dir(&prev).unwrap();

    assert!(config.lanes["local"].source.is_none());

    let ci_source = config.lanes["ci"].source.as_deref();
    assert_eq!(ci_source, Some("CorvidLabs/fledge-lanes"));
    assert_eq!(
        determine_trust_tier(ci_source.unwrap()),
        crate::trust::TrustTier::Official
    );

    let deploy_source = config.lanes["deploy"].source.as_deref();
    assert_eq!(deploy_source, Some("someuser/lanes"));
    assert_eq!(
        determine_trust_tier(deploy_source.unwrap()),
        crate::trust::TrustTier::Unverified
    );
}

#[test]
fn list_lanes_json_includes_trust_tier() {
    let mut lanes = BTreeMap::new();
    lanes.insert(
        "local".to_string(),
        LaneDef {
            description: Some("Local lane".to_string()),
            steps: vec![Step::TaskRef("lint".to_string())],
            fail_fast: true,
            source: None,
        },
    );
    lanes.insert(
        "imported".to_string(),
        LaneDef {
            description: Some("Remote lane".to_string()),
            steps: vec![Step::TaskRef("test".to_string())],
            fail_fast: true,
            source: Some("CorvidLabs/fledge-lanes".to_string()),
        },
    );
    lanes.insert(
        "third_party".to_string(),
        LaneDef {
            description: Some("Third party".to_string()),
            steps: vec![Step::TaskRef("deploy".to_string())],
            fail_fast: true,
            source: Some("someuser/lanes".to_string()),
        },
    );

    let result = list_lanes(&lanes, true);
    assert!(result.is_ok());
}
