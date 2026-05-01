use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

use crate::test_support::cwd_lock;
use crate::versioning::parse_version;

fn with_cwd<F: FnOnce() -> R, R>(dir: &Path, f: F) -> R {
    let _guard = cwd_lock();
    let saved = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let _ = std::env::set_current_dir(saved);
    match result {
        Ok(r) => r,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn init_git_repo(dir: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn commit_file(dir: &Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).unwrap();
    Command::new("git")
        .args(["add", name])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", &format!("add {name}")])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn apply_bump_major() {
    let v = parse_version("1.2.3").unwrap();
    let bumped = version::apply_bump(&v, "major").unwrap();
    assert_eq!(bumped.to_string(), "2.0.0");
}

#[test]
fn apply_bump_minor() {
    let v = parse_version("1.2.3").unwrap();
    let bumped = version::apply_bump(&v, "minor").unwrap();
    assert_eq!(bumped.to_string(), "1.3.0");
}

#[test]
fn apply_bump_patch() {
    let v = parse_version("1.2.3").unwrap();
    let bumped = version::apply_bump(&v, "patch").unwrap();
    assert_eq!(bumped.to_string(), "1.2.4");
}

#[test]
fn apply_bump_from_zero() {
    let v = parse_version("0.0.0").unwrap();
    assert_eq!(
        version::apply_bump(&v, "major").unwrap().to_string(),
        "1.0.0"
    );
    assert_eq!(
        version::apply_bump(&v, "minor").unwrap().to_string(),
        "0.1.0"
    );
    assert_eq!(
        version::apply_bump(&v, "patch").unwrap().to_string(),
        "0.0.1"
    );
}

#[test]
fn apply_bump_invalid_level() {
    let v = parse_version("1.2.3").unwrap();
    assert!(version::apply_bump(&v, "mega").is_err());
}

#[test]
fn extract_toml_version_basic() {
    let content = r#"
[package]
name = "my-app"
version = "0.5.0"
edition = "2021"
"#;
    assert_eq!(
        toml_utils::extract_toml_version(content),
        Some("0.5.0".to_string())
    );
}

#[test]
fn extract_toml_version_not_found() {
    assert_eq!(toml_utils::extract_toml_version("name = \"foo\""), None);
}

#[test]
fn detect_version_files_rust() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"",
    )
    .unwrap();
    let files = bump::detect_version_files(tmp.path());
    assert_eq!(files, vec!["Cargo.toml"]);
}

#[test]
fn detect_version_files_node() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("package.json"), r#"{"version": "1.0.0"}"#).unwrap();
    let files = bump::detect_version_files(tmp.path());
    assert_eq!(files, vec!["package.json"]);
}

#[test]
fn detect_version_files_empty() {
    let tmp = TempDir::new().unwrap();
    let files = bump::detect_version_files(tmp.path());
    assert!(files.is_empty());
}

#[test]
fn detect_version_files_multiple() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("Cargo.toml"), "version = \"0.1.0\"").unwrap();
    fs::write(tmp.path().join("package.json"), "{}").unwrap();
    let files = bump::detect_version_files(tmp.path());
    assert_eq!(files.len(), 2);
}

#[test]
fn classify_conventional_commits() {
    assert_eq!(
        changelog::classify_for_changelog("feat: add release"),
        "Features"
    );
    assert_eq!(
        changelog::classify_for_changelog("fix: handle null"),
        "Fixes"
    );
    assert_eq!(
        changelog::classify_for_changelog("docs: update readme"),
        "Documentation"
    );
    assert_eq!(
        changelog::classify_for_changelog("chore: bump deps"),
        "Chores"
    );
    assert_eq!(
        changelog::classify_for_changelog("feat(cli): add flag"),
        "Features"
    );
    assert_eq!(changelog::classify_for_changelog("random message"), "Other");
}

#[test]
fn strip_prefix_simple() {
    assert_eq!(
        changelog::strip_conventional_prefix("feat: add release"),
        "add release"
    );
    assert_eq!(
        changelog::strip_conventional_prefix("fix(core): null check"),
        "null check"
    );
    assert_eq!(
        changelog::strip_conventional_prefix("update readme"),
        "update readme"
    );
}

#[test]
fn strip_prefix_no_space_after_colon() {
    assert_eq!(
        changelog::strip_conventional_prefix("feat:add release"),
        "add release"
    );
    assert_eq!(
        changelog::strip_conventional_prefix("fix(core):null check"),
        "null check"
    );
}

#[test]
fn read_cargo_version_test() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"3.2.1\"\n",
    )
    .unwrap();
    assert_eq!(version::read_cargo_version(tmp.path()).unwrap(), "3.2.1");
}

#[test]
fn read_package_json_version_test() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "test", "version": "2.0.0"}"#,
    )
    .unwrap();
    assert_eq!(
        version::read_package_json_version(tmp.path()).unwrap(),
        "2.0.0"
    );
}

#[test]
fn read_python_version_test() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("pyproject.toml"),
        "[project]\nname = \"test\"\nversion = \"1.5.0\"\n",
    )
    .unwrap();
    assert_eq!(version::read_python_version(tmp.path()).unwrap(), "1.5.0");
}

#[test]
fn bump_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    let new_ver = parse_version("0.2.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"Cargo.toml".to_string()));
    let content = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
    assert!(content.contains("version = \"0.2.0\""));
}

#[test]
fn bump_package_json() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "package.json",
        r#"{"name": "test", "version": "1.0.0"}"#,
    );
    let new_ver = parse_version("1.1.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"package.json".to_string()));
    let content = fs::read_to_string(tmp.path().join("package.json")).unwrap();
    assert!(content.contains("\"1.1.0\""));
}

#[test]
fn bump_pyproject_toml() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.3.0\"\n",
    );
    let new_ver = parse_version("0.4.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"pyproject.toml".to_string()));
}

#[test]
fn detect_version_from_plugin_toml() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
            tmp.path(),
            "plugin.toml",
            "[plugin]\nname = \"fledge-deploy\"\nversion = \"0.3.0\"\n\n[[commands]]\nname = \"deploy\"\nbinary = \"bin/deploy\"\n",
        );
    let v = version::detect_current_version(tmp.path()).unwrap();
    assert_eq!(v.to_string(), "0.3.0");
}

#[test]
fn bump_plugin_toml_only_touches_plugin_section() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    // Manifest has version inside [plugin] AND a `version` key elsewhere
    // (a `[[commands]]` table) — the bumper must only rewrite the [plugin]
    // one.
    let manifest = "[plugin]\nname = \"fledge-deploy\"\nversion = \"0.1.0\"\n\n[[commands]]\nname = \"deploy\"\nbinary = \"bin/deploy\"\nversion = \"99.99.99\"\n";
    commit_file(tmp.path(), "plugin.toml", manifest);
    let new_ver = parse_version("0.2.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"plugin.toml".to_string()));
    let updated = fs::read_to_string(tmp.path().join("plugin.toml")).unwrap();
    assert!(updated.contains("[plugin]\nname = \"fledge-deploy\"\nversion = \"0.2.0\""));
    // The bogus `version` on the command row stays put.
    assert!(updated.contains("version = \"99.99.99\""));
}

#[test]
fn bump_plugin_toml_with_cargo_toml_keeps_them_in_sync() {
    // Rust plugins (e.g. fledge-plugin-metrics) carry both manifests and
    // expect both to bump together.
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "plugin.toml",
        "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n",
    );
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    );
    let new_ver = parse_version("0.2.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"plugin.toml".to_string()));
    assert!(result.files_bumped.contains(&"Cargo.toml".to_string()));
    let plugin = fs::read_to_string(tmp.path().join("plugin.toml")).unwrap();
    let cargo = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
    assert!(plugin.contains("version = \"0.2.0\""));
    assert!(cargo.contains("version = \"0.2.0\""));
}

#[test]
fn extract_versioned_section_skips_other_tables() {
    let toml = "[plugin]\nname = \"x\"\nversion = \"0.1.0\"\n\n[[commands]]\nname = \"y\"\nversion = \"99.0.0\"\n";
    assert_eq!(
        toml_utils::extract_versioned_toml_section(toml, "plugin"),
        Some("0.1.0".to_string())
    );
}

#[test]
fn extract_versioned_section_returns_none_when_section_absent() {
    let toml = "[plugin]\nname = \"x\"\n";
    assert_eq!(
        toml_utils::extract_versioned_toml_section(toml, "plugin"),
        None
    );
}

#[test]
fn replace_versioned_section_returns_none_when_no_match() {
    let toml = "[other]\nversion = \"1.0.0\"\n";
    assert_eq!(
        toml_utils::replace_versioned_toml_section(toml, "plugin", "2.0.0"),
        None
    );
}

#[test]
fn replace_versioned_section_preserves_trailing_newline() {
    let toml = "[plugin]\nversion = \"0.1.0\"\n";
    let out = toml_utils::replace_versioned_toml_section(toml, "plugin", "0.2.0").unwrap();
    assert_eq!(out, "[plugin]\nversion = \"0.2.0\"\n");
}

#[test]
fn bump_release_files_flake_nix() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    commit_file(
        tmp.path(),
        "flake.nix",
        "{ pname = \"x\"; version = \"0.1.0\"; }\n",
    );
    commit_file(
        tmp.path(),
        "fledge.toml",
        "[release]\nfiles = [\"flake.nix\"]\n",
    );
    let new_ver = parse_version("0.2.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"flake.nix".to_string()));
    let content = fs::read_to_string(tmp.path().join("flake.nix")).unwrap();
    assert!(content.contains("version = \"0.2.0\""));
}

#[test]
fn preflight_checks_not_git() {
    let tmp = TempDir::new().unwrap();
    assert!(preflight_checks(tmp.path(), false).is_err());
}

#[test]
fn preflight_checks_clean() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "test.txt", "hello");
    assert!(preflight_checks(tmp.path(), false).is_ok());
}

#[test]
fn preflight_checks_dirty_allowed() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "test.txt", "hello");
    fs::write(tmp.path().join("dirty.txt"), "untracked").unwrap();
    assert!(preflight_checks(tmp.path(), true).is_ok());
}

#[test]
fn preflight_checks_dirty_blocked() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "test.txt", "hello");
    fs::write(tmp.path().join("dirty.txt"), "untracked").unwrap();
    assert!(preflight_checks(tmp.path(), false).is_err());
}

#[test]
fn resolve_explicit_version() {
    let tmp = TempDir::new().unwrap();
    let v = version::resolve_target_version(tmp.path(), "2.0.0").unwrap();
    assert_eq!(v.to_string(), "2.0.0");
}

#[test]
fn resolve_bump_from_cargo() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"1.0.0\"\n",
    );
    let v = version::resolve_target_version(tmp.path(), "minor").unwrap();
    assert_eq!(v.to_string(), "1.1.0");
}

#[test]
fn dry_run_no_changes() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    let tmp_path = tmp.path().to_path_buf();
    let result = with_cwd(&tmp_path, || {
        run(ReleaseOptions {
            bump: "patch".to_string(),
            dry_run: true,
            no_tag: false,
            no_changelog: false,
            no_bump: false,
            push: false,
            pre_lane: None,
            allow_dirty: false,
            json: false,
        })
    });

    assert!(result.is_ok());
    let content = fs::read_to_string(tmp_path.join("Cargo.toml")).unwrap();
    assert!(content.contains("0.1.0"), "dry run should not modify files");
}

#[test]
fn full_release_flow() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    Command::new("git")
        .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    commit_file(tmp.path(), "src.rs", "fn main() {}");

    let tmp_path = tmp.path().to_path_buf();
    let result = with_cwd(&tmp_path, || {
        run(ReleaseOptions {
            bump: "minor".to_string(),
            dry_run: false,
            no_tag: false,
            no_changelog: false,
            no_bump: false,
            push: false,
            pre_lane: None,
            allow_dirty: false,
            json: false,
        })
    });

    assert!(result.is_ok());

    let content = fs::read_to_string(tmp_path.join("Cargo.toml")).unwrap();
    assert!(content.contains("version = \"0.2.0\""));

    assert!(tmp_path.join("CHANGELOG.md").exists());

    let tag_output = Command::new("git")
        .args(["tag", "-l", "v0.2.0"])
        .current_dir(&tmp_path)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&tag_output.stdout).contains("v0.2.0"));
}

#[test]
fn release_tag_only_project() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "main.go", "package main");

    Command::new("git")
        .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    commit_file(tmp.path(), "main_test.go", "package main");

    let tmp_path = tmp.path().to_path_buf();
    let result = with_cwd(&tmp_path, || {
        run(ReleaseOptions {
            bump: "patch".to_string(),
            dry_run: false,
            no_tag: false,
            no_changelog: false,
            no_bump: false,
            push: false,
            pre_lane: None,
            allow_dirty: false,
            json: false,
        })
    });

    assert!(result.is_ok());

    let tag_output = Command::new("git")
        .args(["tag", "-l", "v0.1.1"])
        .current_dir(&tmp_path)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&tag_output.stdout).contains("v0.1.1"));
}

#[test]
fn changelog_entry_format() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "a.txt", "a");

    Command::new("git")
        .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    fs::write(tmp.path().join("b.txt"), "b").unwrap();
    Command::new("git")
        .args(["add", "b.txt"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: add feature b"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let tmp_path = tmp.path().to_path_buf();
    let version = parse_version("0.2.0").unwrap();
    let result = with_cwd(&tmp_path, || {
        changelog::generate_changelog_entry(&tmp_path, &version, false)
    });

    assert!(result.is_ok());
    let cl = fs::read_to_string(tmp_path.join("CHANGELOG.md")).unwrap();
    assert!(cl.contains("[v0.2.0]"));
    assert!(cl.contains("### Features"));
    assert!(cl.contains("add feature b"));
}

#[test]
fn changelog_appends_to_existing() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());

    let existing = "# Changelog\n\n## [v0.1.0] - 2024-01-01\n\n### Features\n\n- initial\n";
    commit_file(tmp.path(), "CHANGELOG.md", existing);

    Command::new("git")
        .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    fs::write(tmp.path().join("new.txt"), "new").unwrap();
    Command::new("git")
        .args(["add", "new.txt"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "fix: patch bug"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let tmp_path = tmp.path().to_path_buf();
    let version = parse_version("0.1.1").unwrap();
    with_cwd(&tmp_path, || {
        changelog::generate_changelog_entry(&tmp_path, &version, false).unwrap();
    });

    let cl = fs::read_to_string(tmp_path.join("CHANGELOG.md")).unwrap();
    assert!(cl.contains("[v0.1.1]"));
    assert!(cl.contains("[v0.1.0]"));
    let pos_new = cl.find("[v0.1.1]").unwrap();
    let pos_old = cl.find("[v0.1.0]").unwrap();
    assert!(
        pos_new < pos_old,
        "new entry should appear before old entry"
    );
}

#[test]
fn read_maven_version_test() {
    let tmp = TempDir::new().unwrap();
    let pom = r#"<?xml version="1.0"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
    <version>1.3.0</version>
</project>"#;
    fs::write(tmp.path().join("pom.xml"), pom).unwrap();
    assert_eq!(version::read_maven_version(tmp.path()).unwrap(), "1.3.0");
}

#[test]
fn read_gradle_version_test() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("build.gradle.kts"),
        "plugins { id(\"java\") }\nversion = \"2.1.0\"\n",
    )
    .unwrap();
    assert_eq!(version::read_gradle_version(tmp.path()).unwrap(), "2.1.0");
}

#[test]
fn custom_release_files() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());

    let fledge_toml = r#"
[release]
files = ["version.txt"]
"#;
    commit_file(tmp.path(), "fledge.toml", fledge_toml);
    commit_file(tmp.path(), "version.txt", "version = \"0.1.0\"");
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    let new_ver = parse_version("0.2.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"version.txt".to_string()));

    let content = fs::read_to_string(tmp.path().join("version.txt")).unwrap();
    assert!(content.contains("0.2.0"));
}

#[test]
fn read_gemspec_version_test() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("my_gem.gemspec"),
        r#"
Gem::Specification.new do |s|
  s.name = "my_gem"
  s.version = "1.4.2"
  s.summary = "A test gem"
end
"#,
    )
    .unwrap();
    assert_eq!(version::read_gemspec_version(tmp.path()).unwrap(), "1.4.2");
}

#[test]
fn read_gemspec_version_single_quotes() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("my_gem.gemspec"),
        "Gem::Specification.new do |s|\n  s.version = '2.0.1'\nend\n",
    )
    .unwrap();
    assert_eq!(version::read_gemspec_version(tmp.path()).unwrap(), "2.0.1");
}

#[test]
fn read_python_version_from_setup_cfg() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("setup.cfg"),
        "[metadata]\nname = my_pkg\nversion = 3.1.0\n",
    )
    .unwrap();
    assert_eq!(version::read_python_version(tmp.path()).unwrap(), "3.1.0");
}

#[test]
fn read_python_version_pyproject_takes_priority() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("pyproject.toml"),
        "[project]\nname = \"test\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("setup.cfg"),
        "[metadata]\nversion = 2.0.0\n",
    )
    .unwrap();
    assert_eq!(version::read_python_version(tmp.path()).unwrap(), "1.0.0");
}

#[test]
fn duplicate_tag_prevented() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "test.txt", "hello");

    Command::new("git")
        .args(["tag", "-a", "v1.0.0", "-m", "v1.0.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let version = parse_version("1.0.0").unwrap();
    let result = git::create_tag(tmp.path(), &version, false);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("already exists"),
        "expected 'already exists' error, got: {err}"
    );
}

#[test]
fn bump_setup_cfg() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "setup.cfg",
        "[metadata]\nname = test\nversion = 0.5.0\n",
    );
    commit_file(tmp.path(), "pyproject.toml", "[build-system]\n");

    let new_ver = parse_version("0.6.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"setup.cfg".to_string()));
    let content = fs::read_to_string(tmp.path().join("setup.cfg")).unwrap();
    assert!(content.contains("0.6.0"));
}

#[test]
fn bump_pom_xml() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    let pom = r#"<?xml version="1.0"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
    <version>1.0.0</version>
</project>"#;
    commit_file(tmp.path(), "pom.xml", pom);

    let new_ver = parse_version("1.1.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result.files_bumped.contains(&"pom.xml".to_string()));
    let content = fs::read_to_string(tmp.path().join("pom.xml")).unwrap();
    assert!(content.contains("<version>1.1.0</version>"));
}

#[test]
fn bump_gradle() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "build.gradle.kts",
        "plugins { id(\"java\") }\nversion = \"0.3.0\"\n",
    );

    let new_ver = parse_version("0.4.0").unwrap();
    let result = bump::bump_version_files(tmp.path(), &new_ver).unwrap();
    assert!(result
        .files_bumped
        .contains(&"build.gradle.kts".to_string()));
    let content = fs::read_to_string(tmp.path().join("build.gradle.kts")).unwrap();
    assert!(content.contains("\"0.4.0\""));
}

#[test]
fn changelog_created_fresh() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(tmp.path(), "a.txt", "a");

    let tmp_path = tmp.path().to_path_buf();
    let version = parse_version("0.1.0").unwrap();
    with_cwd(&tmp_path, || {
        changelog::generate_changelog_entry(&tmp_path, &version, false).unwrap();
    });

    let cl = fs::read_to_string(tmp_path.join("CHANGELOG.md")).unwrap();
    assert!(cl.starts_with("# Changelog"));
    assert!(cl.contains("[v0.1.0]"));
}

#[test]
fn release_with_no_tag_flag() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    commit_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    Command::new("git")
        .args(["tag", "-a", "v0.1.0", "-m", "v0.1.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    commit_file(tmp.path(), "src.rs", "fn main() {}");

    let tmp_path = tmp.path().to_path_buf();
    let result = with_cwd(&tmp_path, || {
        run(ReleaseOptions {
            bump: "patch".to_string(),
            dry_run: false,
            no_tag: true,
            no_changelog: true,
            no_bump: false,
            push: false,
            pre_lane: None,
            allow_dirty: false,
            json: false,
        })
    });

    assert!(result.is_ok());

    let content = fs::read_to_string(tmp_path.join("Cargo.toml")).unwrap();
    assert!(content.contains("version = \"0.1.1\""));

    let tag_output = Command::new("git")
        .args(["tag", "-l", "v0.1.1"])
        .current_dir(&tmp_path)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&tag_output.stdout)
            .trim()
            .is_empty(),
        "no tag should be created with --no-tag"
    );
}
