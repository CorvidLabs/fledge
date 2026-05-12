use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use super::commands::{init, new_spec};
use super::parse::{extract_purpose, extract_sections, parse_frontmatter};
use super::validation::{validate_spec, SpecResult, ValidationIssue};
use super::*;

#[test]
fn test_parse_frontmatter_valid() {
    let content = r#"---
module: init
version: 4
status: active
files:
  - src/init.rs
  - src/main.rs

db_tables: []
depends_on:
  - templates
---

# Init

## Purpose

Test purpose.
"#;
    let (fm, body) = parse_frontmatter(content).unwrap();
    assert_eq!(fm.module, "init");
    assert_eq!(fm.version, 4);
    assert_eq!(fm.status, "active");
    assert_eq!(fm.files, vec!["src/init.rs", "src/main.rs"]);
    assert!(body.contains("## Purpose"));
}

#[test]
fn test_parse_frontmatter_missing_module() {
    let content = r#"---
version: 1
status: draft
files: []
---
body
"#;
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("module"));
}

#[test]
fn test_parse_frontmatter_missing_version() {
    let content = r#"---
module: test
status: draft
files: []
---
body
"#;
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("version"));
}

#[test]
fn test_parse_frontmatter_missing_status() {
    let content = r#"---
module: test
version: 1
files: []
---
body
"#;
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("status"));
}

#[test]
fn test_parse_frontmatter_no_delimiters() {
    let content = "no frontmatter here";
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("---"));
}

#[test]
fn test_parse_frontmatter_no_closing() {
    let content = "---\nmodule: test\n";
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("closing"));
}

#[test]
fn test_extract_sections() {
    let body = r#"
# Title

## Purpose

Some text.

## Public API

More text.

## Invariants

1. First
"#;
    let sections = extract_sections(body);
    assert_eq!(sections, vec!["Purpose", "Public API", "Invariants"]);
}

#[test]
fn test_extract_sections_empty() {
    let body = "No sections here, just text.";
    let sections = extract_sections(body);
    assert!(sections.is_empty());
}

#[test]
fn test_extract_purpose_happy_path() {
    let body = "\n## Purpose\n\nA short description.\n\n## Public API\n\ntext\n";
    assert_eq!(extract_purpose(body), Some("A short description.".into()));
}

#[test]
fn test_extract_purpose_multiline_joined() {
    let body = "## Purpose\n\nLine one\nline two\n\n## Next\n";
    assert_eq!(extract_purpose(body), Some("Line one line two".into()));
}

#[test]
fn test_extract_purpose_missing_section() {
    let body = "## Public API\n\ntext\n";
    assert_eq!(extract_purpose(body), None);
}

fn scaffold_min_project(tmp: &TempDir, modules: &[&str]) {
    let specsync = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync).unwrap();
    fs::write(
        specsync.join("config.toml"),
        "specs_dir = \"specs\"\nrequired_sections = []\n",
    )
    .unwrap();
    for name in modules {
        let dir = tmp.path().join(format!("specs/{name}"));
        fs::create_dir_all(&dir).unwrap();
        let spec = format!(
            "---\nmodule: {name}\nversion: 1\nstatus: active\nfiles: []\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nPurpose of {name}.\n\n## Public API\n\n## Invariants\n\n## Behavioral Examples\n\n## Error Cases\n\n## Dependencies\n\n## Change Log\n"
        );
        fs::write(dir.join(format!("{name}.spec.md")), spec).unwrap();
        fs::write(dir.join("requirements.md"), "---\nspec: x\n---\nreq body\n").unwrap();
        fs::write(dir.join("context.md"), "---\nspec: x\n---\ncontext body\n").unwrap();
    }
}

#[test]
fn test_collect_index_sorted_with_purpose() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &["zebra", "alpha", "mango"]);

    let entries = collect_index(tmp.path()).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "mango", "zebra"]);
    assert_eq!(entries[0].purpose, Some("Purpose of alpha.".into()));
    assert_eq!(entries[0].version, 1);
    assert_eq!(entries[0].status, "active");
}

#[test]
fn test_collect_index_empty_project() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &[]);
    let entries = collect_index(tmp.path()).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn test_render_index_markdown_contains_entries() {
    let entries = vec![
        IndexEntry {
            name: "foo".into(),
            version: 2,
            status: "active".into(),
            purpose: Some("Does foo.".into()),
            files: vec!["src/foo.rs".into()],
            path: PathBuf::from("specs/foo/foo.spec.md"),
        },
        IndexEntry {
            name: "bar".into(),
            version: 1,
            status: "draft".into(),
            purpose: None,
            files: Vec::new(),
            path: PathBuf::from("specs/bar/bar.spec.md"),
        },
    ];
    let md = render_index_markdown(&entries);
    assert!(md.contains("## Available specs"));
    assert!(md.contains("**foo** v2 (active)"));
    assert!(md.contains("Does foo."));
    assert!(md.contains("**bar** v1 (draft)"));
    assert!(md.contains("(no purpose documented)"));
}

#[test]
fn test_all_module_names_sorted() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &["beta", "alpha"]);
    let names = all_module_names(tmp.path()).unwrap();
    assert_eq!(names, vec!["alpha", "beta"]);
}

#[test]
fn test_load_module_bundle_includes_spec_and_companions() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &["alpha"]);
    let bundle = load_module_bundle(tmp.path(), "alpha").unwrap();
    assert!(bundle.contains("## Spec bundle: alpha"));
    assert!(bundle.contains("### `alpha.spec.md`"));
    assert!(bundle.contains("Purpose of alpha."));
    assert!(bundle.contains("### `requirements.md`"));
    assert!(bundle.contains("req body"));
    assert!(bundle.contains("### `context.md`"));
    assert!(bundle.contains("context body"));
    // tasks and testing not scaffolded, so not present
    assert!(!bundle.contains("### `tasks.md`"));
    assert!(!bundle.contains("### `testing.md`"));
}

#[test]
fn test_load_module_bundle_missing_module_errors() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &[]);
    let err = load_module_bundle(tmp.path(), "ghost").unwrap_err();
    assert!(err.to_string().contains("No spec found"));
}

#[test]
fn test_load_module_bundle_rejects_path_traversal() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &["real"]);

    // `..` (anywhere) and `\\` must always be rejected; leading/trailing `/` too.
    // Note: `foo/bar` is now a legitimate nested name (issue #383) and is only
    // rejected here because no such spec exists in the scaffold.
    for bad in [
        "../evil", "..\\evil", "foo\\bar", "..", ".", "", "/foo", "foo/", "foo//bar",
    ] {
        let err = load_module_bundle(tmp.path(), bad).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid module name") || msg.contains("cannot be empty"),
            "expected rejection for '{bad}', got: {msg}"
        );
    }
}

#[test]
fn test_validate_module_name_allows_normal_names() {
    assert!(validate_module_name("trust").is_ok());
    assert!(validate_module_name("create_template").is_ok());
    assert!(validate_module_name("plugin-protocol").is_ok());
    // Nested names (issue #383)
    assert!(validate_module_name("game/board").is_ok());
    assert!(validate_module_name("network/websocket").is_ok());
}

#[test]
fn test_validate_module_name_rejects_invalid_nested_forms() {
    assert!(validate_module_name("/foo").is_err());
    assert!(validate_module_name("foo/").is_err());
    assert!(validate_module_name("foo//bar").is_err());
    assert!(validate_module_name("foo/../bar").is_err());
    assert!(validate_module_name("foo\\bar").is_err());
    assert!(validate_module_name("./foo").is_err());
}

fn scaffold_project_with_source_specs(tmp: &TempDir) {
    let specsync = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync).unwrap();
    fs::write(
        specsync.join("config.toml"),
        "specs_dir = \"specs\"\nrequired_sections = []\n",
    )
    .unwrap();

    for (name, source_files) in [
        ("trust", vec!["src/trust.rs"]),
        ("ask", vec!["src/ask.rs"]),
        ("work", vec!["src/work.rs"]),
    ] {
        let dir = tmp.path().join(format!("specs/{name}"));
        fs::create_dir_all(&dir).unwrap();
        let files_yaml = source_files
            .iter()
            .map(|f| format!("  - {f}"))
            .collect::<Vec<_>>()
            .join("\n");
        let spec = format!(
            "---\nmodule: {name}\nversion: 1\nstatus: active\nfiles:\n{files_yaml}\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n"
        );
        fs::write(dir.join(format!("{name}.spec.md")), spec).unwrap();
    }
}

#[test]
fn test_specs_for_changed_files_matches_via_frontmatter_files() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_source_specs(&tmp);

    let changed = vec!["src/trust.rs".to_string(), "src/ask.rs".to_string()];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert_eq!(matched, vec!["ask", "trust"]);
}

#[test]
fn test_specs_for_changed_files_matches_via_spec_directory() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_source_specs(&tmp);

    let changed = vec!["specs/trust/context.md".to_string()];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert_eq!(matched, vec!["trust"]);
}

#[test]
fn test_specs_for_changed_files_deduplicates() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_source_specs(&tmp);

    // Both trust.rs and specs/trust/context.md → single match
    let changed = vec![
        "src/trust.rs".to_string(),
        "specs/trust/context.md".to_string(),
    ];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert_eq!(matched, vec!["trust"]);
}

#[test]
fn test_specs_for_changed_files_no_match() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_source_specs(&tmp);

    let changed = vec!["README.md".to_string(), "Cargo.toml".to_string()];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert!(matched.is_empty());
}

#[test]
fn test_specs_for_changed_files_empty_input() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_source_specs(&tmp);
    let matched = specs_for_changed_files(tmp.path(), &[]).unwrap();
    assert!(matched.is_empty());
}

#[test]
fn test_specs_for_changed_files_honors_custom_specs_dir() {
    let tmp = TempDir::new().unwrap();
    let specsync = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync).unwrap();
    fs::write(
        specsync.join("config.toml"),
        "specs_dir = \"docs/specs\"\nrequired_sections = []\n",
    )
    .unwrap();
    let dir = tmp.path().join("docs/specs/trust");
    fs::create_dir_all(&dir).unwrap();
    let spec = "---\nmodule: trust\nversion: 1\nstatus: active\nfiles:\n  - src/trust.rs\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n";
    fs::write(dir.join("trust.spec.md"), spec).unwrap();

    // Match via `docs/specs/trust/...` prefix, not `specs/trust/...`
    let changed = vec!["docs/specs/trust/context.md".to_string()];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert_eq!(matched, vec!["trust"]);

    // Changing a file under the legacy `specs/...` path should NOT match
    // when the project uses a custom specs_dir
    let changed_wrong = vec!["specs/trust/context.md".to_string()];
    let matched_wrong = specs_for_changed_files(tmp.path(), &changed_wrong).unwrap();
    assert!(matched_wrong.is_empty());
}

/// Scaffold a project where two specs share a directory: a primary
/// `plugin/plugin.spec.md` and a nested `plugin/plugin-protocol.spec.md`
/// declaring `module: plugin-protocol`. Mirrors the layout from issue #291.
fn scaffold_project_with_nested_spec(tmp: &TempDir) {
    let specsync = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync).unwrap();
    fs::write(
        specsync.join("config.toml"),
        "specs_dir = \"specs\"\nrequired_sections = []\n",
    )
    .unwrap();
    let dir = tmp.path().join("specs/plugin");
    fs::create_dir_all(&dir).unwrap();

    let plugin_spec = "---\nmodule: plugin\nversion: 1\nstatus: active\nfiles:\n  - src/plugin.rs\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n";
    fs::write(dir.join("plugin.spec.md"), plugin_spec).unwrap();

    let protocol_spec = "---\nmodule: plugin-protocol\nversion: 1\nstatus: active\nfiles:\n  - src/protocol.rs\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nProtocol purpose.\n";
    fs::write(dir.join("plugin-protocol.spec.md"), protocol_spec).unwrap();
    fs::write(dir.join("requirements.md"), "shared reqs\n").unwrap();
}

#[test]
fn test_specs_for_changed_files_resolves_nested_spec() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_nested_spec(&tmp);

    // Changing src/protocol.rs (declared in plugin-protocol's `files:`)
    // should resolve via the nested spec, not via a non-existent
    // specs/plugin-protocol/ directory.
    let changed = vec!["src/protocol.rs".to_string()];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert_eq!(matched, vec!["plugin-protocol"]);
}

#[test]
fn test_specs_for_changed_files_shared_dir_matches_both_specs() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_nested_spec(&tmp);

    // A file under specs/plugin/ affects both specs that share the dir.
    let changed = vec!["specs/plugin/requirements.md".to_string()];
    let matched = specs_for_changed_files(tmp.path(), &changed).unwrap();
    assert_eq!(matched, vec!["plugin", "plugin-protocol"]);
}

#[test]
fn test_load_module_bundle_loads_nested_spec_by_module_name() {
    let tmp = TempDir::new().unwrap();
    scaffold_project_with_nested_spec(&tmp);

    let bundle = load_module_bundle(tmp.path(), "plugin-protocol").unwrap();
    assert!(bundle.contains("## Spec bundle: plugin-protocol"));
    assert!(bundle.contains("### `plugin-protocol.spec.md`"));
    assert!(bundle.contains("Protocol purpose."));
    // Companions in the shared parent dir are picked up.
    assert!(bundle.contains("### `requirements.md`"));
    assert!(bundle.contains("shared reqs"));
}

#[test]
fn test_validate_spec_all_valid() {
    let tmp = TempDir::new().unwrap();
    let specs_dir = tmp.path().join("specs/mymod");
    fs::create_dir_all(&specs_dir).unwrap();

    let src_file = tmp.path().join("src/mymod.rs");
    fs::create_dir_all(src_file.parent().unwrap()).unwrap();
    fs::write(&src_file, "// source").unwrap();

    for companion in &["requirements.md", "tasks.md", "context.md", "testing.md"] {
        fs::write(specs_dir.join(companion), "---\nspec: mymod.spec.md\n---\n").unwrap();
    }

    let spec_content = r#"---
module: mymod
version: 1
status: active
files:
  - src/mymod.rs
db_tables: []
depends_on: []
---

# Mymod

## Purpose
Test

## Public API
Test

## Invariants
Test

## Behavioral Examples
Test

## Error Cases
Test

## Dependencies
Test

## Change Log
Test
"#;
    let spec_path = specs_dir.join("mymod.spec.md");
    fs::write(&spec_path, spec_content).unwrap();

    let required = vec![
        "Purpose".to_string(),
        "Public API".to_string(),
        "Invariants".to_string(),
        "Behavioral Examples".to_string(),
        "Error Cases".to_string(),
        "Dependencies".to_string(),
        "Change Log".to_string(),
    ];

    let result = validate_spec(&spec_path, tmp.path(), &required);
    assert_eq!(result.name, "mymod");
    assert_eq!(result.version, 1);
    assert_eq!(result.status, "active");
    assert!(!result.has_errors());
    assert!(!result.has_warnings());
}

#[test]
fn test_validate_spec_missing_file() {
    let tmp = TempDir::new().unwrap();
    let specs_dir = tmp.path().join("specs/mymod");
    fs::create_dir_all(&specs_dir).unwrap();

    let spec_content = r#"---
module: mymod
version: 1
status: active
files:
  - src/nonexistent.rs
db_tables: []
depends_on: []
---

# Mymod

## Purpose
## Public API
## Invariants
## Behavioral Examples
## Error Cases
## Dependencies
## Change Log
"#;
    let spec_path = specs_dir.join("mymod.spec.md");
    fs::write(&spec_path, spec_content).unwrap();

    let required = vec![
        "Purpose".to_string(),
        "Public API".to_string(),
        "Invariants".to_string(),
        "Behavioral Examples".to_string(),
        "Error Cases".to_string(),
        "Dependencies".to_string(),
        "Change Log".to_string(),
    ];

    let result = validate_spec(&spec_path, tmp.path(), &required);
    assert!(result.has_errors());
    assert!(result
        .issues
        .iter()
        .any(|i| i.message.contains("file not found")));
}

#[test]
fn test_validate_spec_missing_sections() {
    let tmp = TempDir::new().unwrap();
    let specs_dir = tmp.path().join("specs/mymod");
    fs::create_dir_all(&specs_dir).unwrap();

    let spec_content = r#"---
module: mymod
version: 1
status: active
files: []
db_tables: []
depends_on: []
---

# Mymod

## Purpose
Test

## Public API
Test
"#;
    let spec_path = specs_dir.join("mymod.spec.md");
    fs::write(&spec_path, spec_content).unwrap();

    let required = vec![
        "Purpose".to_string(),
        "Public API".to_string(),
        "Invariants".to_string(),
    ];

    let result = validate_spec(&spec_path, tmp.path(), &required);
    assert!(result.has_errors());
    assert!(result
        .issues
        .iter()
        .any(|i| i.message.contains("Invariants")));
}

#[test]
fn test_validate_spec_missing_companion() {
    let tmp = TempDir::new().unwrap();
    let specs_dir = tmp.path().join("specs/mymod");
    fs::create_dir_all(&specs_dir).unwrap();

    let spec_content = r#"---
module: mymod
version: 1
status: active
files: []
db_tables: []
depends_on: []
---

# Mymod

## Purpose
## Public API
## Invariants
## Behavioral Examples
## Error Cases
## Dependencies
## Change Log
"#;
    let spec_path = specs_dir.join("mymod.spec.md");
    fs::write(&spec_path, spec_content).unwrap();

    let required = vec![
        "Purpose".to_string(),
        "Public API".to_string(),
        "Invariants".to_string(),
        "Behavioral Examples".to_string(),
        "Error Cases".to_string(),
        "Dependencies".to_string(),
        "Change Log".to_string(),
    ];

    let result = validate_spec(&spec_path, tmp.path(), &required);
    assert!(!result.has_errors());
    assert!(result.has_warnings());
    assert!(result
        .issues
        .iter()
        .any(|i| i.message.contains("companion file missing")));
}

#[test]
fn test_validate_spec_invalid_status() {
    let tmp = TempDir::new().unwrap();
    let specs_dir = tmp.path().join("specs/mymod");
    fs::create_dir_all(&specs_dir).unwrap();

    let spec_content = r#"---
module: mymod
version: 1
status: banana
files: []
db_tables: []
depends_on: []
---

# Mymod

## Purpose
## Public API
## Invariants
## Behavioral Examples
## Error Cases
## Dependencies
## Change Log
"#;
    let spec_path = specs_dir.join("mymod.spec.md");
    fs::write(&spec_path, spec_content).unwrap();

    let required = vec![
        "Purpose".to_string(),
        "Public API".to_string(),
        "Invariants".to_string(),
        "Behavioral Examples".to_string(),
        "Error Cases".to_string(),
        "Dependencies".to_string(),
        "Change Log".to_string(),
    ];

    let result = validate_spec(&spec_path, tmp.path(), &required);
    assert!(result.has_errors());
    assert!(result
        .issues
        .iter()
        .any(|i| i.message.contains("Invalid status")));
}

#[test]
fn test_to_title_case() {
    assert_eq!(to_title_case("hello_world"), "Hello World");
    assert_eq!(to_title_case("auth"), "Auth");
    assert_eq!(to_title_case("create_template"), "Create Template");
}

#[test]
fn test_init_creates_files() {
    let tmp = TempDir::new().unwrap();

    let result = init(tmp.path());

    assert!(result.is_ok());
    assert!(tmp.path().join(".specsync/config.toml").exists());
    assert!(tmp.path().join(".specsync/registry.toml").exists());
    assert!(tmp.path().join(".specsync/.gitignore").exists());
    assert!(tmp.path().join(".specsync/version").exists());
    assert!(tmp.path().join("specs").exists());
}

#[test]
fn test_init_refuses_existing() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join(".specsync")).unwrap();

    let result = init(tmp.path());

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_new_spec_creates_files() {
    let tmp = TempDir::new().unwrap();

    let specsync_dir = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync_dir).unwrap();
    fs::write(
        specsync_dir.join("config.toml"),
        "specs_dir = \"specs\"\nrequired_sections = []\n",
    )
    .unwrap();
    fs::write(
        specsync_dir.join("registry.toml"),
        "[registry]\nname = \"test\"\n\n[specs]\n",
    )
    .unwrap();

    let result = new_spec(tmp.path(), "auth");

    assert!(result.is_ok());
    assert!(tmp.path().join("specs/auth/auth.spec.md").exists());
    assert!(tmp.path().join("specs/auth/requirements.md").exists());
    assert!(tmp.path().join("specs/auth/tasks.md").exists());
    assert!(tmp.path().join("specs/auth/context.md").exists());
    assert!(tmp.path().join("specs/auth/testing.md").exists());

    let registry = fs::read_to_string(specsync_dir.join("registry.toml")).unwrap();
    assert!(registry.contains("auth = \"specs/auth/auth.spec.md\""));
}

#[test]
fn test_new_spec_supports_nested_names() {
    let tmp = TempDir::new().unwrap();

    let specsync_dir = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync_dir).unwrap();
    fs::write(
        specsync_dir.join("config.toml"),
        "specs_dir = \"specs\"\nrequired_sections = []\n",
    )
    .unwrap();
    fs::write(
        specsync_dir.join("registry.toml"),
        "[registry]\nname = \"test\"\n\n[specs]\n",
    )
    .unwrap();

    let result = new_spec(tmp.path(), "game/board");

    assert!(result.is_ok(), "{result:?}");
    assert!(tmp.path().join("specs/game/board/board.spec.md").exists());
    assert!(tmp.path().join("specs/game/board/requirements.md").exists());
    assert!(tmp.path().join("specs/game/board/tasks.md").exists());
    assert!(tmp.path().join("specs/game/board/context.md").exists());
    assert!(tmp.path().join("specs/game/board/testing.md").exists());

    let spec = fs::read_to_string(tmp.path().join("specs/game/board/board.spec.md")).unwrap();
    assert!(spec.contains("module: game/board"));
    assert!(spec.contains("src/board.rs"));

    let req = fs::read_to_string(tmp.path().join("specs/game/board/requirements.md")).unwrap();
    assert!(req.contains("spec: board.spec.md"));

    let registry = fs::read_to_string(specsync_dir.join("registry.toml")).unwrap();
    assert!(registry.contains("\"game/board\" = \"specs/game/board/board.spec.md\""));
}

#[test]
fn test_new_spec_refuses_existing() {
    let tmp = TempDir::new().unwrap();

    let specsync_dir = tmp.path().join(".specsync");
    fs::create_dir_all(&specsync_dir).unwrap();
    fs::write(
        specsync_dir.join("config.toml"),
        "specs_dir = \"specs\"\nrequired_sections = []\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("specs/auth")).unwrap();

    let result = new_spec(tmp.path(), "auth");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_spec_result_counts() {
    let result = SpecResult {
        name: "test".to_string(),
        version: 1,
        status: "active".to_string(),
        file_count: 1,
        section_count: 7,
        required_count: 7,
        issues: vec![
            ValidationIssue {
                message: "error1".to_string(),
                is_error: true,
            },
            ValidationIssue {
                message: "warn1".to_string(),
                is_error: false,
            },
            ValidationIssue {
                message: "warn2".to_string(),
                is_error: false,
            },
        ],
    };
    assert_eq!(result.error_count(), 1);
    assert_eq!(result.warning_count(), 2);
    assert!(result.has_errors());
    assert!(result.has_warnings());
}

#[test]
fn test_parse_frontmatter_inline_files() {
    let content = r#"---
module: test
version: 1
status: draft
files: [src/a.rs, src/b.rs]
db_tables: []
depends_on: []
---

body
"#;
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(fm.files, vec!["src/a.rs", "src/b.rs"]);
}
