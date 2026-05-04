use std::collections::BTreeMap;

use crate::trust::determine_trust_tier;

use super::{
    base64_decode, create_lane_repo, evaluate_when, execute_lane, format_duration,
    format_lane_toml, lane_defaults, list_lanes, load_lane_config, parse_import_source,
    resolve_from, validate_lane, validate_lanes, FledgeFileWithLanes, LaneDef, ParallelItem, Step,
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
        Step::Inline { run: cmd, .. } => assert_eq!(cmd, "cargo build --release"),
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
        Step::Parallel { parallel, .. } => {
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
        None,
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
        None,
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
        None,
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
        Step::Parallel { parallel, .. } => {
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
        None,
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
        None,
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
            when: None,
            timeout: None,
            retries: None,
            retry_delay: None,
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
        None,
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
        None,
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
        None,
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
            when: None,
            timeout: None,
            retries: None,
            retry_delay: None,
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
            when: None,
            timeout: None,
            retries: None,
            retry_delay: None,
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

    let _guard = crate::test_support::cwd_lock();
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

// ── --from flag ──────────────────────────────────────────────────────

#[test]
fn resolve_from_by_index() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"
b = "echo b"
c = "echo c"

[lanes.ci]
steps = ["a", "b", "c"]
"#,
    );
    let idx = resolve_from(&config.lanes["ci"].steps, "2").unwrap();
    assert_eq!(idx, 1); // 0-based
}

#[test]
fn resolve_from_by_name() {
    let config = parse_config(
        r#"
[tasks]
lint = "echo lint"
test = "echo test"
build = "echo build"

[lanes.ci]
steps = ["lint", "test", "build"]
"#,
    );
    let idx = resolve_from(&config.lanes["ci"].steps, "test").unwrap();
    assert_eq!(idx, 1);
}

#[test]
fn resolve_from_index_out_of_range() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"

[lanes.ci]
steps = ["a"]
"#,
    );
    let result = resolve_from(&config.lanes["ci"].steps, "5");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of range"));
}

#[test]
fn resolve_from_index_zero() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"

[lanes.ci]
steps = ["a"]
"#,
    );
    let result = resolve_from(&config.lanes["ci"].steps, "0");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of range"));
}

#[test]
fn resolve_from_unknown_name() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"

[lanes.ci]
steps = ["a"]
"#,
    );
    let result = resolve_from(&config.lanes["ci"].steps, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not match"));
}

#[test]
fn execute_from_skips_earlier_steps() {
    let config = parse_config(
        r#"
[tasks]
a = "exit 1"
b = "echo ok-b"
c = "echo ok-c"

[lanes.ci]
steps = ["a", "b", "c"]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    // Step "a" would fail, but --from 2 skips it
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        Some(1), // 0-based index for step 2
    );
    assert!(result.is_ok());
}

#[test]
fn execute_from_by_name_skips() {
    let config = parse_config(
        r#"
[tasks]
fail = "exit 1"
ok = "echo ok"

[lanes.ci]
steps = ["fail", "ok"]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    // Skip "fail", start from "ok"
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        Some(1),
    );
    assert!(result.is_ok());
}

// ── when conditions ──────────────────────────────────────────────────
//
// These tests use `evaluate_when_with` and a `HashMap` to avoid mutating
// process-global env vars. Rust's test runner is multi-threaded, and on
// edition 2024 `std::env::set_var` is `unsafe` due to soundness issues
// with concurrent setenv/getenv calls. Routing the lookup through a
// closure side-steps both concerns.

fn env_map<I, K, V>(pairs: I) -> std::collections::HashMap<String, String>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    pairs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
}

fn eval(condition: &str, env: &std::collections::HashMap<String, String>) -> bool {
    super::evaluate_when_with(condition, |var| env.get(var).cloned())
}

#[test]
fn evaluate_when_var_set() {
    let env = env_map([("FLEDGE_TEST_WHEN_SET", "1")]);
    assert!(eval("FLEDGE_TEST_WHEN_SET", &env));
}

#[test]
fn evaluate_when_var_not_set() {
    let env = env_map::<_, &str, &str>([]);
    assert!(!eval("FLEDGE_TEST_WHEN_UNSET", &env));
}

#[test]
fn evaluate_when_var_set_but_empty() {
    let env = env_map([("FLEDGE_TEST_WHEN_EMPTY", "")]);
    // Empty value is treated as not-set per the bare-VAR semantics
    assert!(!eval("FLEDGE_TEST_WHEN_EMPTY", &env));
}

#[test]
fn evaluate_when_var_equals() {
    let env = env_map([("FLEDGE_TEST_WHEN_EQ", "true")]);
    assert!(eval("FLEDGE_TEST_WHEN_EQ=true", &env));
    assert!(!eval("FLEDGE_TEST_WHEN_EQ=false", &env));
}

#[test]
fn evaluate_when_negated_var() {
    let unset = env_map::<_, &str, &str>([]);
    assert!(eval("!FLEDGE_TEST_WHEN_NEG", &unset));
    let set = env_map([("FLEDGE_TEST_WHEN_NEG", "1")]);
    assert!(!eval("!FLEDGE_TEST_WHEN_NEG", &set));
}

#[test]
fn evaluate_when_negated_equals() {
    let env = env_map([("FLEDGE_TEST_WHEN_NEQ", "prod")]);
    assert!(eval("!FLEDGE_TEST_WHEN_NEQ=dev", &env));
    assert!(!eval("!FLEDGE_TEST_WHEN_NEQ=prod", &env));
}

#[test]
fn evaluate_when_multiple_conditions() {
    let env = env_map([("FLEDGE_TEST_A", "1"), ("FLEDGE_TEST_B", "2")]);
    assert!(eval("FLEDGE_TEST_A,FLEDGE_TEST_B", &env));
    assert!(eval("FLEDGE_TEST_A=1,FLEDGE_TEST_B=2", &env));
    assert!(!eval("FLEDGE_TEST_A=1,FLEDGE_TEST_B=3", &env));
}

#[test]
fn evaluate_when_empty_string() {
    let env = env_map::<_, &str, &str>([]);
    assert!(eval("", &env));
    assert!(eval(",", &env));
}

#[test]
fn evaluate_when_real_env_smoke() {
    // One smoke test that goes through the real `evaluate_when` (which
    // reads `std::env::var`) to confirm the wrapper still works. Uses a
    // var that's basically always present; falls back if not.
    if std::env::var("PATH").is_ok() {
        assert!(evaluate_when("PATH"));
    }
    // A definitely-not-set var name
    assert!(!evaluate_when("FLEDGE_DEFINITELY_NOT_SET_XYZ_8675309"));
}

#[test]
fn parse_when_on_inline_step() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo deploy", when = "CI=true" }]
"#,
    );
    match &config.lanes["ci"].steps[0] {
        Step::Inline { run, when, .. } => {
            assert_eq!(run, "echo deploy");
            assert_eq!(when.as_deref(), Some("CI=true"));
        }
        _ => panic!("expected inline step"),
    }
}

#[test]
fn parse_task_ref_full_with_when() {
    let config = parse_config(
        r#"
[tasks]
deploy = "echo deploy"

[lanes.ci]
steps = [{ task = "deploy", when = "CI=true" }]
"#,
    );
    match &config.lanes["ci"].steps[0] {
        Step::TaskRefFull { task, when, .. } => {
            assert_eq!(task, "deploy");
            assert_eq!(when.as_deref(), Some("CI=true"));
        }
        _ => panic!("expected TaskRefFull step"),
    }
}

#[test]
fn execute_when_skips_step() {
    std::env::remove_var("FLEDGE_TEST_SKIP");
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [
  { run = "exit 1", when = "FLEDGE_TEST_SKIP=yes" },
  { run = "echo passed" },
]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    // First step has when condition that's not met, so it's skipped
    // Second step runs and passes
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_ok());
}

#[test]
fn execute_when_runs_step() {
    std::env::set_var("FLEDGE_TEST_RUN", "yes");
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo conditional-ok", when = "FLEDGE_TEST_RUN=yes" }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_ok());
    std::env::remove_var("FLEDGE_TEST_RUN");
}

// ── timeout ──────────────────────────────────────────────────────────

#[test]
fn parse_timeout_on_inline() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo fast", timeout = 30 }]
"#,
    );
    assert_eq!(config.lanes["ci"].steps[0].timeout(), Some(30));
}

#[test]
fn parse_retries_on_inline() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo flaky", retries = 3 }]
"#,
    );
    assert_eq!(config.lanes["ci"].steps[0].retries(), Some(3));
}

#[test]
fn parse_all_options_on_task_ref_full() {
    let config = parse_config(
        r#"
[tasks]
deploy = "echo deploy"

[lanes.ci]
steps = [{ task = "deploy", when = "CI", timeout = 60, retries = 2 }]
"#,
    );
    let step = &config.lanes["ci"].steps[0];
    assert_eq!(step.when(), Some("CI"));
    assert_eq!(step.timeout(), Some(60));
    assert_eq!(step.retries(), Some(2));
}

#[test]
fn execute_timeout_kills_slow_command() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "sleep 30", timeout = 1 }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    // The lane failed at step 1 within ~1s, proving the timeout killed
    // the 30s sleep. The error message mentions "step 1" and the elapsed
    // time is around 1s, not 30s.
    assert!(
        err.contains("failed at step 1"),
        "expected failure at step 1, got: {err}"
    );
}

#[test]
fn execute_timeout_fast_command_succeeds() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo fast", timeout = 30 }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_ok());
}

// ── retries ──────────────────────────────────────────────────────────

#[test]
fn execute_retries_succeed_on_first_try() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo ok", retries = 3 }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_ok());
}

#[test]
fn execute_retries_still_fail_after_exhaustion() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "exit 1", retries = 2 }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "ci",
        &config.lanes["ci"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_err());
}

#[test]
fn parse_retry_delay_on_inline() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [{ run = "echo flaky", retries = 3, retry_delay = 5 }]
"#,
    );
    assert_eq!(config.lanes["ci"].steps[0].retries(), Some(3));
    assert_eq!(config.lanes["ci"].steps[0].retry_delay(), Some(5));
}

#[test]
fn execute_retry_delay_zero_skips_sleep() {
    // With retry_delay = 0 the retry loop sleeps 0s between attempts —
    // failed-after-exhaustion completes essentially instantly, proving
    // the delay value is honored (default 1s would make this take ~2s).
    let config = parse_config(
        r#"
[tasks]

[lanes.fast-fail]
steps = [{ run = "exit 1", retries = 2, retry_delay = 0 }]
"#,
    );
    let project_dir = std::env::current_dir().unwrap();
    let start = std::time::Instant::now();
    let result = execute_lane(
        "fast-fail",
        &config.lanes["fast-fail"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    let elapsed = start.elapsed();
    assert!(result.is_err());
    assert!(
        elapsed < std::time::Duration::from_millis(800),
        "expected near-instant fail with retry_delay=0, took {elapsed:?}"
    );
}

#[cfg(unix)]
#[test]
fn execute_retries_succeed_on_third_attempt() {
    // Core value prop of retries: a flaky step that fails twice, succeeds
    // on the third attempt. Uses a counter file as a side channel between
    // attempts to coordinate state. With retries = 2 the lane runs the
    // step 3 times total — first two fail, third succeeds.
    let counter = std::env::temp_dir().join(format!(
        "fledge_test_retry_counter_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let _ = std::fs::remove_file(&counter);
    let counter_path = counter.display().to_string();

    let cmd = format!(
        "n=$(cat {p} 2>/dev/null || echo 0); n=$((n+1)); echo $n > {p}; \
         if [ $n -ge 3 ]; then exit 0; else exit 1; fi",
        p = counter_path
    );

    let toml_str = format!(
        r#"
[tasks]

[lanes.flaky]
steps = [{{ run = "{cmd}", retries = 2 }}]
"#
    );
    let config = parse_config(&toml_str);
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "flaky",
        &config.lanes["flaky"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    let final_count = std::fs::read_to_string(&counter)
        .unwrap_or_default()
        .trim()
        .to_string();
    let _ = std::fs::remove_file(&counter);

    assert!(
        result.is_ok(),
        "expected success after retries, got: {:?}",
        result.err().map(|e| e.to_string())
    );
    assert_eq!(
        final_count, "3",
        "expected 3 attempts (initial + 2 retries), counter shows: {final_count}"
    );
}

// ── combined features ────────────────────────────────────────────────

#[test]
fn parse_parallel_with_when() {
    let config = parse_config(
        r#"
[tasks]
a = "echo a"
b = "echo b"

[lanes.ci]
steps = [{ parallel = ["a", "b"], when = "CI" }]
"#,
    );
    match &config.lanes["ci"].steps[0] {
        Step::Parallel { parallel, when, .. } => {
            assert_eq!(parallel.len(), 2);
            assert_eq!(when.as_deref(), Some("CI"));
        }
        _ => panic!("expected parallel step"),
    }
}

#[test]
fn validate_task_ref_full_unknown() {
    let config = parse_config(
        r#"
[tasks]
lint = "echo lint"

[lanes.ci]
steps = [{ task = "nonexistent", when = "CI" }]
"#,
    );
    let result = validate_lane("ci", &config.lanes["ci"], &config.tasks);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nonexistent"));
}

#[test]
fn format_lane_toml_with_task_ref_full() {
    let lane = LaneDef {
        description: Some("CI".to_string()),
        steps: vec![Step::TaskRefFull {
            task: "deploy".to_string(),
            when: Some("CI=true".to_string()),
            timeout: Some(60),
            retries: Some(2),
            retry_delay: None,
        }],
        fail_fast: true,
        source: None,
    };
    let toml = format_lane_toml("ci", &lane);
    assert!(toml.contains("task = \"deploy\""));
    assert!(toml.contains("when = \"CI=true\""));
    assert!(toml.contains("timeout = 60"));
    assert!(toml.contains("retries = 2"));
}

#[test]
fn format_lane_toml_inline_with_extras() {
    let lane = LaneDef {
        description: None,
        steps: vec![Step::Inline {
            run: "echo hi".to_string(),
            when: Some("CI".to_string()),
            timeout: None,
            retries: Some(1),
            retry_delay: None,
        }],
        fail_fast: true,
        source: None,
    };
    let toml = format_lane_toml("test", &lane);
    assert!(toml.contains("run = \"echo hi\""));
    assert!(toml.contains("when = \"CI\""));
    assert!(toml.contains("retries = 1"));
    assert!(!toml.contains("timeout"));
}

#[test]
fn bare_task_ref_has_no_options() {
    let config = parse_config(
        r#"
[tasks]
lint = "echo lint"

[lanes.ci]
steps = ["lint"]
"#,
    );
    let step = &config.lanes["ci"].steps[0];
    assert!(step.when().is_none());
    assert!(step.timeout().is_none());
    assert!(step.retries().is_none());
}

#[test]
fn resolve_from_with_inline_step() {
    let config = parse_config(
        r#"
[tasks]

[lanes.ci]
steps = [
  { run = "echo first" },
  { run = "echo second" },
]
"#,
    );
    let idx = resolve_from(&config.lanes["ci"].steps, "echo second").unwrap();
    assert_eq!(idx, 1);
}

#[test]
fn resolve_from_with_task_ref_full() {
    let config = parse_config(
        r#"
[tasks]
deploy = "echo deploy"

[lanes.ci]
steps = [
  "deploy",
  { task = "deploy", when = "CI" },
]
"#,
    );
    // First match wins — "deploy" matches step 0 (bare TaskRef)
    let idx = resolve_from(&config.lanes["ci"].steps, "deploy").unwrap();
    assert_eq!(idx, 0);
}

#[test]
fn resolve_from_parallel_item_emits_specific_error() {
    let config = parse_config(
        r#"
[tasks]
lint = "echo lint"
fmt = "echo fmt"
build = "echo build"

[lanes.ci]
steps = [
  { parallel = ["lint", "fmt"] },
  "build",
]
"#,
    );
    let result = resolve_from(&config.lanes["ci"].steps, "lint");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("parallel"),
        "expected parallel-specific error, got: {err}"
    );
    assert!(
        err.contains("--from 1"),
        "expected hint to use --from <index>, got: {err}"
    );
}

#[test]
fn resolve_from_bare_step_wins_over_parallel_match() {
    // If a bare step also matches the name, that takes precedence over the
    // parallel-only match. Regression guard for the new branch.
    let config = parse_config(
        r#"
[tasks]
lint = "echo lint"
fmt = "echo fmt"

[lanes.ci]
steps = [
  { parallel = ["lint", "fmt"] },
  "lint",
]
"#,
    );
    let idx = resolve_from(&config.lanes["ci"].steps, "lint").unwrap();
    assert_eq!(idx, 1);
}

#[cfg(unix)]
#[test]
fn execute_timeout_kills_grandchild_processes() {
    // Multi-statement shell forks a child that survives kill of `sh` itself
    // unless we kill the entire process group. The marker file is touched
    // only if `sleep` outlives the timeout — which it shouldn't.
    let marker = std::env::temp_dir().join(format!(
        "fledge_test_orphan_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let _ = std::fs::remove_file(&marker);
    let cmd = format!("echo start && sleep 3 && touch {}", marker.display());

    let toml_str = format!(
        r#"
[tasks]

[lanes.timeout]
steps = [{{ run = "{cmd}", timeout = 1 }}]
"#
    );
    let config = parse_config(&toml_str);
    let project_dir = std::env::current_dir().unwrap();
    let result = execute_lane(
        "timeout",
        &config.lanes["timeout"],
        &config.tasks,
        &project_dir,
        false,
        None,
    );
    assert!(result.is_err(), "expected timeout failure");

    // Wait past the original sleep duration. If the process group kill
    // worked, the marker is never created. Otherwise it shows up after ~3s.
    std::thread::sleep(std::time::Duration::from_secs(4));
    let leaked = marker.exists();
    let _ = std::fs::remove_file(&marker);
    assert!(
        !leaked,
        "marker file was created — grandchild sleep was orphaned, not killed"
    );
}
