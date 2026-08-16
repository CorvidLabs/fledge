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

// ── lint: shared fixtures ────────────────────────────────────────────────────

use super::lint::{
    apply_ignores, build_envelope, build_quality_prompt, contains_placeholder_word,
    ensure_provider_available, frontmatter_block_has_items, frontmatter_value, grade_spec,
    initial_model_pass, is_known_check, lint_structural, model_pass_skip_reason,
    normalize_check_id, parse_ignore_list, parse_quality_response, resolve_signal_section,
    resolve_targets, section_has_content, strip_code_spans, strip_html_comments, version_is_valid,
    Finding, Layer, LintOptions, ModelPass, Severity, SpecLintResult, SpecMeta,
    ACCEPTANCE_SECTIONS, REJECTION_SECTIONS,
};

fn required() -> Vec<String> {
    DEFAULT_REQUIRED_SECTIONS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// A spec that passes every layer-1 check. Individual tests substitute one
/// piece of it so each assertion isolates a single check.
fn healthy_spec() -> String {
    r#"---
module: demo
version: 3
status: active
files:
  - src/demo.rs
---

# Demo

## Purpose

Demo turns a parsed config into a validated execution plan, so a malformed
config fails before any task runs.

## Public API

| Export | Description |
|--------|-------------|
| `run` | Executes the validated plan |

## Invariants

1. `run` never mutates the config it was given.

## Behavioral Examples

Given a config with one task, when `run` is called, then the task executes once.

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| UnknownTask | the named task is absent | exits 1 and names the task |

## Dependencies

- serde

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-01-01 | Initial spec |
"#
    .to_string()
}

/// Tempdir containing `src/demo.rs` so the `files:` existence check passes.
fn lint_root() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/demo.rs"), "// demo\n").unwrap();
    tmp
}

fn checks_of(findings: &[Finding]) -> Vec<&str> {
    findings.iter().map(|f| f.check.as_str()).collect()
}

// ── lint layer 1: whole-spec outcomes ────────────────────────────────────────

#[test]
fn test_lint_healthy_spec_has_no_findings() {
    let tmp = lint_root();
    let (meta, findings) = lint_structural(&healthy_spec(), tmp.path(), &required());
    assert_eq!(meta.name.as_deref(), Some("demo"));
    assert_eq!(meta.version.as_deref(), Some("3"));
    assert_eq!(meta.status.as_deref(), Some("active"));
    assert!(findings.is_empty(), "unexpected findings: {findings:?}");
}

#[test]
fn test_lint_reports_missing_frontmatter() {
    let tmp = lint_root();
    let (_, findings) = lint_structural("# No frontmatter\n", tmp.path(), &required());
    assert_eq!(checks_of(&findings), vec!["frontmatter"]);
}

#[test]
fn test_lint_reports_missing_source_file() {
    let tmp = lint_root();
    let spec = healthy_spec().replace("  - src/demo.rs", "  - src/deleted.rs");
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    assert_eq!(checks_of(&findings), vec!["missing_file"]);
    assert!(findings[0].message.contains("src/deleted.rs"));
}

#[test]
fn test_lint_reports_missing_required_section() {
    let tmp = lint_root();
    let spec = healthy_spec().replace("## Invariants", "## Notes");
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    assert_eq!(checks_of(&findings), vec!["missing_section"]);
    assert_eq!(findings[0].section.as_deref(), Some("Invariants"));
}

#[test]
fn test_lint_reports_empty_section_of_pure_scaffolding() {
    let tmp = lint_root();
    let spec = healthy_spec().replace(
        "1. `run` never mutates the config it was given.",
        "1. <!-- List invariants that must always hold. -->",
    );
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    assert_eq!(checks_of(&findings), vec!["empty_section"]);
    assert_eq!(findings[0].section.as_deref(), Some("Invariants"));
}

#[test]
fn test_lint_reports_placeholder_tokens_in_purpose_and_public_api() {
    let tmp = lint_root();
    let spec = healthy_spec()
        .replace(
            "Demo turns a parsed config into a validated execution plan, so a malformed",
            "TODO: describe this. It also does things, so a malformed",
        )
        .replace("| `run` | Executes the validated plan |", "| `run` | TBD |");
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    assert_eq!(
        checks_of(&findings),
        vec!["placeholder_text", "placeholder_text"]
    );
    assert_eq!(findings[0].section.as_deref(), Some("Purpose"));
    assert_eq!(findings[1].section.as_deref(), Some("Public API"));
}

#[test]
fn test_lint_reports_lowercase_and_mixed_case_placeholder_tokens() {
    let tmp = lint_root();
    for placeholder in ["todo: describe this.", "Todo: describe this.", "FixMe."] {
        let spec = healthy_spec().replace(
            "Demo turns a parsed config into a validated execution plan, so a malformed",
            &format!("{placeholder} It also does things, so a malformed"),
        );
        let (_, findings) = lint_structural(&spec, tmp.path(), &required());
        assert!(
            checks_of(&findings).contains(&"placeholder_text"),
            "{placeholder:?} should fire placeholder_text: {findings:?}"
        );
        assert_eq!(findings[0].section.as_deref(), Some("Purpose"));
    }
}

#[test]
fn test_lint_does_not_flag_placeholder_tokens_inside_longer_words() {
    // Case-insensitive matching without word boundaries would flag ordinary
    // prose: "mastodon" contains "todo", "prefixmethod" contains "fixme".
    let tmp = lint_root();
    let spec = healthy_spec().replace(
        "Demo turns a parsed config into a validated execution plan, so a malformed",
        "Demo posts the mastodon feed through its prefixmethod hook, so a malformed",
    );
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    assert!(
        !checks_of(&findings).contains(&"placeholder_text"),
        "{findings:?}"
    );
}

#[test]
fn test_lint_accepts_integer_and_semver_versions_but_rejects_others() {
    let tmp = lint_root();
    for good in ["3", "1.2.3", "0.1.0-rc.1"] {
        let spec = healthy_spec().replace("version: 3", &format!("version: {good}"));
        let (_, findings) = lint_structural(&spec, tmp.path(), &required());
        assert!(
            !checks_of(&findings).contains(&"version_format"),
            "{good} should be accepted, got {findings:?}"
        );
    }
    for bad in ["v3", "1.2", "draft"] {
        let spec = healthy_spec().replace("version: 3", &format!("version: {bad}"));
        let (_, findings) = lint_structural(&spec, tmp.path(), &required());
        assert!(
            checks_of(&findings).contains(&"version_format"),
            "{bad} should be rejected, got {findings:?}"
        );
    }
}

#[test]
fn test_lint_semver_version_still_yields_the_rest_of_the_frontmatter() {
    // The typed parser wants a u32; lint normalizes the version first so a
    // semver spec doesn't lose its `files:` checks as collateral damage.
    let tmp = lint_root();
    let spec = healthy_spec()
        .replace("version: 3", "version: 1.2.3")
        .replace("  - src/demo.rs", "  - src/deleted.rs");
    let (meta, findings) = lint_structural(&spec, tmp.path(), &required());
    assert_eq!(meta.version.as_deref(), Some("1.2.3"));
    assert_eq!(checks_of(&findings), vec!["missing_file"]);
}

#[test]
fn test_lint_reports_missing_acceptance_and_rejection_signals() {
    let tmp = lint_root();
    let spec = healthy_spec()
        .replace(
            "Given a config with one task, when `run` is called, then the task executes once.",
            "<!-- pending -->",
        )
        .replace(
            "| UnknownTask | the named task is absent | exits 1 and names the task |",
            "| | | |",
        );
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    let checks = checks_of(&findings);
    assert!(checks.contains(&"no_acceptance_signal"), "{findings:?}");
    assert!(checks.contains(&"no_rejection_signal"), "{findings:?}");
    // Emptiness and "no signal" are separate facts; both are reported.
    assert_eq!(checks.iter().filter(|c| **c == "empty_section").count(), 2);
}

#[test]
fn test_lint_accepts_and_rejects_frontmatter_blocks_supply_the_signals() {
    let tmp = lint_root();
    let spec = healthy_spec()
        .replace(
            "files:\n  - src/demo.rs",
            "files:\n  - src/demo.rs\naccepts:\n  - a valid config produces a plan\nrejects:\n  - an unknown task name exits 1",
        )
        .replace(
            "Given a config with one task, when `run` is called, then the task executes once.",
            "<!-- pending -->",
        )
        .replace(
            "| UnknownTask | the named task is absent | exits 1 and names the task |",
            "| | | |",
        );
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    let checks = checks_of(&findings);
    assert!(!checks.contains(&"no_acceptance_signal"), "{findings:?}");
    assert!(!checks.contains(&"no_rejection_signal"), "{findings:?}");
}

#[test]
fn test_lint_check_ids_are_a_closed_vocabulary() {
    // Guards `--ignore`: an id nothing can emit would silently suppress nothing.
    for check in super::lint::STRUCTURAL_CHECKS {
        assert!(is_known_check(check));
    }
    for check in super::lint::MODEL_CHECKS {
        assert!(is_known_check(check));
    }
    assert!(is_known_check("model_pass_failed"));
    assert!(!is_known_check("not_a_check"));
}

// ── lint layer 1: configurable `required_sections` ───────────────────────────

/// The reviewer's scenario: a project that renames every default section via
/// `.specsync/config.toml`'s `required_sections`.
fn custom_required() -> Vec<String> {
    [
        "Overview", "API", "Rules", "Examples", "Failures", "Deps", "History",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// [`healthy_spec`] rewritten under [`custom_required`]'s section names.
fn custom_spec() -> String {
    let mut spec = healthy_spec();
    for (default, custom) in [
        ("## Purpose", "## Overview"),
        ("## Public API", "## API"),
        ("## Invariants", "## Rules"),
        ("## Behavioral Examples", "## Examples"),
        ("## Error Cases", "## Failures"),
        ("## Dependencies", "## Deps"),
        ("## Change Log", "## History"),
    ] {
        spec = spec.replace(default, custom);
    }
    spec
}

#[test]
fn test_lint_placeholder_check_honors_custom_required_sections() {
    // Regression: the check scanned a hardcoded "Purpose"/"Public API" pair, so
    // a project that renamed them got zero placeholder findings — the gate was
    // silently inert exactly where it was configured to matter.
    let tmp = lint_root();
    let spec = custom_spec().replace(
        "Demo turns a parsed config into a validated execution plan, so a malformed",
        "TODO: fill this in later. It builds a plan, so a malformed",
    );
    let (_, findings) = lint_structural(&spec, tmp.path(), &custom_required());
    let placeholders: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.check == "placeholder_text")
        .collect();
    assert_eq!(placeholders.len(), 1, "{findings:?}");
    assert_eq!(placeholders[0].section.as_deref(), Some("Overview"));
}

#[test]
fn test_lint_placeholder_check_covers_every_required_section() {
    // Widening from the hardcoded pair to the configured set also closes the
    // hole under the *default* config: a placeholder in Invariants used to pass.
    let tmp = lint_root();
    let spec = healthy_spec().replace(
        "1. `run` never mutates the config it was given.",
        "1. FIXME: work out what run actually guarantees.",
    );
    let (_, findings) = lint_structural(&spec, tmp.path(), &required());
    let placeholders: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.check == "placeholder_text")
        .collect();
    assert_eq!(placeholders.len(), 1, "{findings:?}");
    assert_eq!(placeholders[0].section.as_deref(), Some("Invariants"));
}

#[test]
fn test_lint_resolves_signal_sections_through_recognized_synonyms() {
    // Regression: `no_acceptance_signal` / `no_rejection_signal` looked only for
    // the literal "Behavioral Examples" / "Error Cases", so a project whose
    // Examples and Failures sections were full still failed both checks — a
    // permanent red nothing in the project could clear.
    let tmp = lint_root();
    let (_, findings) = lint_structural(&custom_spec(), tmp.path(), &custom_required());
    assert!(findings.is_empty(), "{findings:?}");

    assert_eq!(
        resolve_signal_section(&custom_required(), ACCEPTANCE_SECTIONS),
        Some("Examples")
    );
    assert_eq!(
        resolve_signal_section(&custom_required(), REJECTION_SECTIONS),
        Some("Failures")
    );
    // Table order is preference order: the canonical name wins over a synonym.
    let both = vec!["Examples".to_string(), "Behavioral Examples".to_string()];
    assert_eq!(
        resolve_signal_section(&both, ACCEPTANCE_SECTIONS),
        Some("Behavioral Examples")
    );
    // Matching ignores ASCII case.
    let shouty = vec!["ERROR CASES".to_string()];
    assert_eq!(
        resolve_signal_section(&shouty, REJECTION_SECTIONS),
        Some("ERROR CASES")
    );
}

#[test]
fn test_lint_reports_missing_signal_against_the_configured_section_name() {
    // The finding must name the project's own heading, not the default one.
    let tmp = lint_root();
    let spec = custom_spec()
        .replace(
            "Given a config with one task, when `run` is called, then the task executes once.",
            "<!-- pending -->",
        )
        .replace(
            "| UnknownTask | the named task is absent | exits 1 and names the task |",
            "| | | |",
        );
    let (_, findings) = lint_structural(&spec, tmp.path(), &custom_required());
    let acceptance = findings
        .iter()
        .find(|f| f.check == "no_acceptance_signal")
        .expect("acceptance finding");
    assert_eq!(acceptance.severity, Severity::Error);
    assert_eq!(acceptance.section.as_deref(), Some("Examples"));
    assert!(acceptance.message.contains("## Examples"), "{acceptance:?}");

    let rejection = findings
        .iter()
        .find(|f| f.check == "no_rejection_signal")
        .expect("rejection finding");
    assert_eq!(rejection.severity, Severity::Error);
    assert_eq!(rejection.section.as_deref(), Some("Failures"));
    assert!(rejection.message.contains("## Failures"), "{rejection:?}");
}

#[test]
fn test_lint_degrades_to_a_warning_when_no_section_carries_a_signal() {
    // A project whose `required_sections` genuinely has no acceptance/rejection
    // analogue must not be handed an unsatisfiable error. The check stays
    // visible as a warning that names the escape hatch.
    let tmp = lint_root();
    let required = vec!["Overview".to_string(), "Notes".to_string()];
    let spec = custom_spec().replace("## Rules", "## Notes");
    let (_, findings) = lint_structural(&spec, tmp.path(), &required);

    for (check, key) in [
        ("no_acceptance_signal", "accepts:"),
        ("no_rejection_signal", "rejects:"),
    ] {
        let finding = findings
            .iter()
            .find(|f| f.check == check)
            .unwrap_or_else(|| panic!("{check} missing from {findings:?}"));
        assert_eq!(finding.severity, Severity::Warning, "{finding:?}");
        // No section is named, because none was identified.
        assert_eq!(finding.section, None, "{finding:?}");
        assert!(finding.message.contains(key), "{finding:?}");
        assert!(
            finding.message.contains("required_sections"),
            "the diagnostic must say why it could not resolve: {finding:?}"
        );
    }
}

#[test]
fn test_lint_frontmatter_blocks_clear_the_unresolvable_signal_warning() {
    // The degraded warning is satisfiable: `accepts:`/`rejects:` work regardless
    // of how a project names its sections.
    let tmp = lint_root();
    let required = vec!["Overview".to_string(), "Notes".to_string()];
    let spec = custom_spec()
        .replace("## Rules", "## Notes")
        .replace(
            "files:\n  - src/demo.rs",
            "files:\n  - src/demo.rs\naccepts:\n  - a valid config produces a plan\nrejects:\n  - an unknown task name exits 1",
        );
    let (_, findings) = lint_structural(&spec, tmp.path(), &required);
    let checks = checks_of(&findings);
    assert!(!checks.contains(&"no_acceptance_signal"), "{findings:?}");
    assert!(!checks.contains(&"no_rejection_signal"), "{findings:?}");
}

#[test]
fn test_required_sections_falls_back_to_the_defaults() {
    // One shared helper decides the fallback, so `check`, `list`, and `lint`
    // cannot drift on what "structurally complete" means.
    let empty = SpecSyncConfig {
        specs_dir: None,
        required_sections: Vec::new(),
    };
    assert_eq!(super::required_sections_of(&empty), required());

    let overridden = SpecSyncConfig {
        specs_dir: None,
        required_sections: custom_required(),
    };
    assert_eq!(super::required_sections_of(&overridden), custom_required());
}

// ── lint layer 1: helpers ────────────────────────────────────────────────────

#[test]
fn test_lint_does_not_flag_backticked_placeholder_tokens() {
    // A spec that *documents* placeholder detection (this module's own spec)
    // must not fail its own check. Prose `TODO` fires; a citation does not.
    let tmp = lint_root();
    let cited = healthy_spec().replace(
        "Demo turns a parsed config into a validated execution plan, so a malformed",
        "Rejects a `TODO` / `TBD` / `FIXME` placeholder. Demo is a plan builder, so a malformed",
    );
    let (_, findings) = lint_structural(&cited, tmp.path(), &required());
    assert!(
        !checks_of(&findings).contains(&"placeholder_text"),
        "{findings:?}"
    );

    let bare = healthy_spec().replace(
        "Demo turns a parsed config into a validated execution plan, so a malformed",
        "TODO write this. Demo is a plan builder, so a malformed",
    );
    let (_, findings) = lint_structural(&bare, tmp.path(), &required());
    assert!(checks_of(&findings).contains(&"placeholder_text"));
}

#[test]
fn test_lint_ignores_placeholder_tokens_in_code_regardless_of_case() {
    // Case-insensitive matching must not reach into code: a lowercase `todo`
    // in a backticked span or a fenced block is still a citation, not a
    // placeholder.
    let tmp = lint_root();
    let cited = healthy_spec().replace(
        "Demo turns a parsed config into a validated execution plan, so a malformed",
        "Rejects `todo`, `Tbd` and `FixMe` markers.\n\n```\ntodo: not prose\n```\n\nDemo is a plan builder, so a malformed",
    );
    let (_, findings) = lint_structural(&cited, tmp.path(), &required());
    assert!(
        !checks_of(&findings).contains(&"placeholder_text"),
        "{findings:?}"
    );
}

#[test]
fn test_contains_placeholder_word_is_case_insensitive_and_word_bounded() {
    // Case folding.
    for prose in ["TODO: x", "todo: x", "Todo: x", "tOdO: x"] {
        assert!(contains_placeholder_word(prose, "TODO"), "{prose:?}");
    }
    assert!(contains_placeholder_word("FixMe soon", "FIXME"));
    assert!(contains_placeholder_word("size is tbd", "TBD"));

    // Word boundaries: punctuation and line edges bound a word, letters/digits
    // and `_` do not.
    for prose in ["(TODO)", "a TODO.", "TODO", "- todo\n", "TODO-ish"] {
        assert!(contains_placeholder_word(prose, "TODO"), "{prose:?}");
    }
    for prose in ["mastodon", "todos", "aTODO", "TODO_LIST", "todo9"] {
        assert!(!contains_placeholder_word(prose, "TODO"), "{prose:?}");
    }
    assert!(!contains_placeholder_word("prefixmethod", "FIXME"));
    assert!(!contains_placeholder_word("subtbdir", "TBD"));

    // Absent, and needle longer than haystack.
    assert!(!contains_placeholder_word("all done", "TODO"));
    assert!(!contains_placeholder_word("hi", "FIXME"));
}

#[test]
fn test_strip_code_spans_removes_fences_and_inline_spans() {
    assert_eq!(strip_code_spans("a `b` c\n").trim(), "a  c");
    assert_eq!(
        strip_code_spans("keep\n```\nTODO\n```\nkeep2\n"),
        "keep\nkeep2\n"
    );
    // Double-backtick spans and an unmatched run are both handled.
    assert_eq!(strip_code_spans("x ``a`b`` y\n").trim(), "x  y");
    assert!(strip_code_spans("unmatched ` TODO\n").contains("TODO"));
    // HTML comments survive: `<!-- TODO -->` is a placeholder, not a citation.
    assert!(strip_code_spans("<!-- TODO -->\n").contains("TODO"));
}

#[test]
fn test_strip_html_comments_handles_multiline_and_unterminated() {
    assert_eq!(strip_html_comments("a <!-- x --> b"), "a  b");
    assert_eq!(strip_html_comments("a <!-- x\ny\n--> b"), "a  b");
    assert_eq!(strip_html_comments("a <!-- never closed"), "a ");
}

#[test]
fn test_section_has_content_ignores_scaffolding() {
    assert!(!section_has_content("\n\n"));
    assert!(!section_has_content("<!-- Describe the module. -->"));
    assert!(!section_has_content("1. <!-- list them -->"));
    assert!(!section_has_content("- "));
    assert!(!section_has_content("```\n```"));
    // A table header + separator is structure; an empty data row adds nothing.
    assert!(!section_has_content(
        "| Error | When | Behavior |\n|-------|------|----------|\n| | | |"
    ));
    assert!(section_has_content("- None"));
    assert!(section_has_content(
        "| Error | When | Behavior |\n|-------|------|----------|\n| Bad | always | exits 1 |"
    ));
}

#[test]
fn test_version_is_valid() {
    assert!(version_is_valid("1"));
    assert!(version_is_valid("42"));
    assert!(version_is_valid("1.2.3"));
    assert!(version_is_valid("1.2.3-rc.1+build.5"));
    assert!(!version_is_valid(""));
    assert!(!version_is_valid("v1"));
    assert!(!version_is_valid("1.2"));
    assert!(!version_is_valid("1.2.3.4"));
}

#[test]
fn test_frontmatter_value_reads_top_level_keys_only() {
    let yaml = "module: demo\nversion: 3\nfiles:\n  - version: nope\n";
    assert_eq!(frontmatter_value(yaml, "module"), Some("demo"));
    assert_eq!(frontmatter_value(yaml, "version"), Some("3"));
    assert_eq!(frontmatter_value(yaml, "missing"), None);
}

#[test]
fn test_frontmatter_block_has_items() {
    assert!(frontmatter_block_has_items(
        "accepts:\n  - one\n",
        "accepts"
    ));
    assert!(frontmatter_block_has_items("accepts: yes\n", "accepts"));
    assert!(!frontmatter_block_has_items("accepts:\n", "accepts"));
    assert!(!frontmatter_block_has_items("accepts: []\n", "accepts"));
    assert!(!frontmatter_block_has_items(
        "files:\n  - a.rs\n",
        "accepts"
    ));
}

// ── lint: ignore list (the human override) ───────────────────────────────────

#[test]
fn test_parse_ignore_list_splits_dedupes_and_validates() {
    let parsed = parse_ignore_list(&[
        "missing_file,empty_section".to_string(),
        " MISSING_FILE ".to_string(),
    ])
    .unwrap();
    assert_eq!(parsed, vec!["empty_section", "missing_file"]);
    let err = parse_ignore_list(&["nope".to_string()]).unwrap_err();
    assert!(err.to_string().contains("unknown --ignore check 'nope'"));
}

#[test]
fn test_apply_ignores_drops_matching_findings_and_counts_them() {
    let tmp = lint_root();
    let spec = healthy_spec().replace("  - src/demo.rs", "  - src/deleted.rs");
    let (_, mut findings) = lint_structural(&spec, tmp.path(), &required());
    let dropped = apply_ignores(&mut findings, &["missing_file".to_string()]);
    assert_eq!(dropped, 1);
    assert!(findings.is_empty());
}

// ── lint layer 2: prompt, parsing, and graceful degradation ──────────────────

#[test]
fn test_build_quality_prompt_grades_the_spec_not_the_code() {
    let spec = healthy_spec();
    let prompt = build_quality_prompt("demo", &spec);
    assert!(prompt.contains("QUALITY OF A SPECIFICATION"));
    assert!(prompt.contains("module `demo`"));
    assert!(prompt.contains("Falsifiable purpose"));
    assert!(prompt.contains("decorative_invariants"));
    // The structural pre-pass owns those checks; the model must not duplicate them.
    assert!(prompt.contains("Do NOT report"));
    assert!(prompt.contains(&spec));
}

#[test]
fn test_parse_quality_response_accepts_fenced_and_prose_wrapped_json() {
    let raw = "Sure!\n```json\n{\"findings\": [{\"check\": \"decorative_invariants\", \
                \"severity\": \"error\", \"section\": \"Invariants\", \
                \"message\": \"Invariant 1 restates the API.\"}]}\n```\n";
    let findings = parse_quality_response(raw).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].check, "decorative_invariants");
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(findings[0].layer, Layer::Model);
    assert_eq!(findings[0].section.as_deref(), Some("Invariants"));
}

#[test]
fn test_parse_quality_response_defaults_severity_and_normalizes_unknown_checks() {
    let raw = r#"{"findings": [{"check": "made_up", "message": "vague purpose"}]}"#;
    let findings = parse_quality_response(raw).unwrap();
    assert_eq!(findings[0].check, "quality_other");
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn test_parse_quality_response_drops_empty_messages_and_rejects_non_json() {
    let findings =
        parse_quality_response(r#"{"findings": [{"check": "api_drift", "message": "  "}]}"#)
            .unwrap();
    assert!(findings.is_empty());
    assert!(parse_quality_response("I could not review that.").is_err());
    assert!(parse_quality_response("{not json}").is_err());
}

#[test]
fn test_normalize_check_id_maps_onto_the_allowlist() {
    assert_eq!(normalize_check_id("API Drift"), "api_drift");
    assert_eq!(
        normalize_check_id("purpose_not_falsifiable"),
        "purpose_not_falsifiable"
    );
    assert_eq!(normalize_check_id("something else"), "quality_other");
}

fn lint_options(ai: bool, no_ai: bool) -> LintOptions {
    LintOptions {
        target: None,
        json: false,
        strict: false,
        ai,
        no_ai,
        provider: None,
        model: None,
        ignore: Vec::new(),
    }
}

#[test]
fn test_model_pass_is_skipped_unless_requested() {
    // Default: layer 2 off, so `spec lint` stays a safe offline pre-commit gate.
    assert!(model_pass_skip_reason(&lint_options(false, false), 3)
        .unwrap()
        .contains("not requested"));
    // --no-ai wins over --ai.
    assert!(model_pass_skip_reason(&lint_options(true, true), 3)
        .unwrap()
        .contains("overrides"));
    assert!(model_pass_skip_reason(&lint_options(false, true), 3).is_some());
    // Nothing to grade.
    assert!(model_pass_skip_reason(&lint_options(true, false), 0).is_some());
    // Requested with specs present: it runs.
    assert!(model_pass_skip_reason(&lint_options(true, false), 3).is_none());
}

#[test]
fn test_model_pass_reports_ai_as_requested_even_when_no_ai_overrides_it() {
    // Regression: `requested` was `ai && !no_ai`, so `--ai --no-ai` emitted
    // `requested: false` alongside `skipped_reason: "--no-ai overrides --ai"` —
    // a self-contradictory object in which a caller auditing `requested` alone
    // could not tell "never asked" from "asked, then overridden".
    let overridden = initial_model_pass(&lint_options(true, true), 3);
    assert!(overridden.requested, "--ai was explicitly passed");
    assert!(!overridden.ran, "--no-ai wins, so the pass never happened");
    assert!(overridden
        .skipped_reason
        .as_deref()
        .unwrap()
        .contains("--no-ai overrides --ai"));

    // Never asked: `requested` is the discriminator between the two states.
    let untouched = initial_model_pass(&lint_options(false, false), 3);
    assert!(!untouched.requested);
    assert!(!untouched.ran);

    // Plain --no-ai is also "not requested".
    assert!(!initial_model_pass(&lint_options(false, true), 3).requested);
    // Asked for, nothing blocking it: still just a request until `ran` is set.
    let granted = initial_model_pass(&lint_options(true, false), 3);
    assert!(granted.requested);
    assert!(granted.skipped_reason.is_none());
}

#[test]
fn test_envelope_reports_the_overridden_request_faithfully() {
    let model_pass = initial_model_pass(&lint_options(true, true), 1);
    let envelope = build_envelope(&[lint_result_fixture()], &model_pass, false);
    let reported = &envelope["model_pass"];
    assert_eq!(reported["requested"], serde_json::json!(true));
    assert_eq!(reported["ran"], serde_json::json!(false));
    assert_eq!(
        reported["skipped_reason"],
        serde_json::json!("--no-ai overrides --ai")
    );
}

#[test]
fn test_ensure_provider_available_errors_when_the_selected_provider_has_no_key() {
    let _lock = crate::test_support::env_lock();
    let _key = crate::test_support::EnvVarGuard::set("ANTHROPIC_API_KEY", None);
    let config = crate::config::Config::default();
    let err = ensure_provider_available(&config, Some("anthropic")).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no API key"), "{msg}");
    assert!(msg.contains("ANTHROPIC_API_KEY"), "{msg}");
    // The escape hatch is named, so the failure is actionable rather than a hang.
    assert!(msg.contains("--ai"), "{msg}");
}

fn lint_result_fixture() -> SpecLintResult {
    SpecLintResult {
        name: "demo".to_string(),
        path: "specs/demo/demo.spec.md".to_string(),
        meta: SpecMeta {
            name: Some("demo".to_string()),
            version: Some("3".to_string()),
            status: Some("active".to_string()),
        },
        findings: Vec::new(),
        ignored: 0,
        content: healthy_spec(),
    }
}

#[test]
fn test_grade_spec_folds_model_findings_into_the_result() {
    let provider = crate::test_support::StubLlmProvider::ok(
        crate::llm::ProviderKind::Ollama,
        Some("stub"),
        r#"{"findings": [{"check": "purpose_not_falsifiable", "severity": "warning", "message": "Purpose restates the module name."}]}"#,
    );
    let mut result = lint_result_fixture();
    grade_spec(&provider, &mut result, &[]);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].layer, Layer::Model);
    assert_eq!(result.warning_count(), 1);
}

#[test]
fn test_grade_spec_turns_a_provider_failure_into_a_model_pass_failed_error() {
    // A gate that could not run must never read as a pass.
    let provider = crate::test_support::StubLlmProvider::err(
        crate::llm::ProviderKind::Ollama,
        Some("stub"),
        "connection refused",
    );
    let mut result = lint_result_fixture();
    grade_spec(&provider, &mut result, &[]);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].check, "model_pass_failed");
    assert_eq!(result.findings[0].severity, Severity::Error);
    assert!(result.findings[0].message.contains("connection refused"));
}

#[test]
fn test_grade_spec_honors_the_ignore_list() {
    let provider = crate::test_support::StubLlmProvider::err(
        crate::llm::ProviderKind::Ollama,
        Some("stub"),
        "boom",
    );
    let mut result = lint_result_fixture();
    grade_spec(&provider, &mut result, &["model_pass_failed".to_string()]);
    assert!(result.findings.is_empty());
    assert_eq!(result.ignored, 1);
}

// ── lint: envelope + target resolution ───────────────────────────────────────

fn quiet_model_pass() -> ModelPass {
    ModelPass {
        requested: false,
        ran: false,
        skipped_reason: Some("not requested (pass --ai for the model-graded pass)".to_string()),
        provider: None,
        model: None,
    }
}

#[test]
fn test_build_envelope_shape_and_pass_verdict() {
    let clean = lint_result_fixture();
    let env = build_envelope(std::slice::from_ref(&clean), &quiet_model_pass(), false);
    assert_eq!(env["schema_version"], 1);
    assert_eq!(env["action"], "spec_lint");
    assert_eq!(env["passed"], true);
    assert_eq!(env["strict"], false);
    assert_eq!(env["totals"]["linted"], 1);
    assert_eq!(env["totals"]["errors"], 0);
    assert_eq!(env["totals"]["ignored"], 0);
    assert_eq!(env["model_pass"]["ran"], false);
    assert_eq!(env["model_pass"]["provider"], serde_json::Value::Null);
    assert_eq!(env["specs"][0]["name"], "demo");
    assert_eq!(env["specs"][0]["version"], "3");
    assert_eq!(env["specs"][0]["passed"], true);
    assert_eq!(env["specs"][0]["findings"], serde_json::json!([]));
}

#[test]
fn test_build_envelope_fails_on_errors_and_on_strict_warnings() {
    let mut failing = lint_result_fixture();
    failing
        .findings
        .push(Finding::error("missing_file", None, "gone"));
    let env = build_envelope(std::slice::from_ref(&failing), &quiet_model_pass(), false);
    assert_eq!(env["passed"], false);
    assert_eq!(env["totals"]["errors"], 1);
    assert_eq!(env["specs"][0]["findings"][0]["check"], "missing_file");
    assert_eq!(env["specs"][0]["findings"][0]["severity"], "error");
    assert_eq!(env["specs"][0]["findings"][0]["layer"], "structural");

    let mut warned = lint_result_fixture();
    warned.findings.push(Finding {
        check: "quality_other".to_string(),
        severity: Severity::Warning,
        layer: Layer::Model,
        section: None,
        message: "thin".to_string(),
    });
    let lax = build_envelope(std::slice::from_ref(&warned), &quiet_model_pass(), false);
    assert_eq!(lax["passed"], true);
    let strict = build_envelope(std::slice::from_ref(&warned), &quiet_model_pass(), true);
    assert_eq!(strict["passed"], false);
}

#[test]
fn test_resolve_targets_by_module_name_path_and_directory() {
    let tmp = TempDir::new().unwrap();
    scaffold_min_project(&tmp, &["alpha", "beta"]);
    let root = tmp.path();

    let all = resolve_targets(root, None).unwrap();
    assert_eq!(all.len(), 2);

    let by_name = resolve_targets(root, Some("alpha")).unwrap();
    assert_eq!(by_name.len(), 1);
    assert!(by_name[0].ends_with("alpha.spec.md"));

    let by_path = resolve_targets(root, Some("specs/beta/beta.spec.md")).unwrap();
    assert_eq!(by_path.len(), 1);
    assert!(by_path[0].ends_with("beta.spec.md"));

    let by_dir = resolve_targets(root, Some("specs/beta")).unwrap();
    assert_eq!(by_dir.len(), 1);

    assert!(resolve_targets(root, Some("nope")).is_err());
}
