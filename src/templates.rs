use anyhow::{bail, Context, Result};
use include_dir::{include_dir, Dir};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use tera::Tera;
use walkdir::WalkDir;

static EMBEDDED_TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Per-command JSON schema versions. Each constant tracks the wire shape of one
/// `templates` subcommand's `--json` envelope independently so that future shape
/// changes can bump exactly the affected envelope without semantically
/// corrupting the meaning of `schema_version` for unrelated commands. Additive
/// changes (new optional fields) do not bump.
pub(crate) const TEMPLATES_LIST_SCHEMA: u32 = 1;
pub(crate) const TEMPLATES_SEARCH_SCHEMA: u32 = 1;
pub(crate) const TEMPLATES_PUBLISH_SCHEMA: u32 = 1;

#[derive(Debug, Deserialize)]
pub struct TemplateManifest {
    pub template: TemplateInfo,
    /// Prompt definitions keyed by variable name. `BTreeMap` so iteration order is
    /// deterministic (alphabetical by key) — locking this for the templates v1
    /// contract means multi-prompt templates ask questions in a stable order
    /// across runs.
    #[serde(default)]
    pub prompts: BTreeMap<String, PromptDef>,
    #[serde(default)]
    pub files: FileRules,
    #[serde(default)]
    pub hooks: Hooks,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Hooks {
    #[serde(default)]
    pub post_create: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub min_fledge_version: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

pub fn check_requirements(requires: &[String]) -> (Vec<String>, Vec<String>) {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for tool in requires {
        if tool.is_empty()
            || tool.contains('/')
            || tool.contains('\\')
            || tool.contains('\0')
            || tool.starts_with('-')
        {
            missing.push(tool.clone());
            continue;
        }
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        let ok = std::process::Command::new(which_cmd)
            .arg(tool)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            found.push(tool.clone());
        } else {
            missing.push(tool.clone());
        }
    }
    (found, missing)
}

#[derive(Debug, Deserialize)]
pub struct PromptDef {
    pub message: String,
    pub default: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FileRules {
    #[serde(default)]
    pub render: Vec<String>,
    /// Globs whose matches are copied verbatim (never run through Tera, even when
    /// they would otherwise match a `render` glob). Use this to mark binary or
    /// otherwise non-templated assets (`**/*.png`, `**/*.ico`, …) so a broad
    /// `render = ["**/*"]` cannot accidentally corrupt them. A `.tera`
    /// extension still wins — that's the explicit "render this" signal.
    #[serde(default)]
    pub copy: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
}

#[derive(Debug)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub manifest: TemplateManifest,
    pub source: Option<String>,
}

pub fn discover_templates(extra_paths: &[PathBuf]) -> Result<Vec<Template>> {
    let mut templates = Vec::new();

    // Built-in templates bundled with fledge
    let builtin_dir = builtin_template_dir();
    if builtin_dir.exists() {
        load_templates_from_dir(&builtin_dir, &mut templates)?;
    }

    // Extra template directories from config
    for path in extra_paths {
        if path.exists() {
            load_templates_from_dir(path, &mut templates)?;
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

pub fn discover_templates_with_repos(
    extra_paths: &[PathBuf],
    repos: &[String],
    token: Option<&str>,
) -> Result<Vec<Template>> {
    let mut templates = discover_templates(extra_paths)?;

    for repo_ref in repos {
        if !crate::remote::is_remote_ref(repo_ref) {
            continue;
        }
        let (owner, repo, subpath, git_ref) = crate::remote::parse_remote_ref(repo_ref)?;
        let before_count = templates.len();
        match crate::remote::resolve_template_dir(owner, repo, subpath, token, git_ref) {
            Ok(dir) => {
                if dir.join("template.toml").exists() {
                    load_single_template(&dir, &mut templates)?;
                } else {
                    load_templates_from_dir(&dir, &mut templates)?;
                }
                for t in templates[before_count..].iter_mut() {
                    t.source = Some(repo_ref.clone());
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to fetch {}: {}", repo_ref, e);
            }
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

fn load_single_template(path: &Path, templates: &mut Vec<Template>) -> Result<()> {
    let manifest_path = path.join("template.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: TemplateManifest =
        toml::from_str(&content).with_context(|| format!("parsing {}", manifest_path.display()))?;
    templates.push(Template {
        name: manifest.template.name.clone(),
        description: manifest.template.description.clone(),
        path: path.to_path_buf(),
        manifest,
        source: None,
    });
    Ok(())
}

fn builtin_template_dir() -> PathBuf {
    // When running from cargo, use the manifest dir; otherwise use exe-relative path
    let exe = std::env::current_exe().unwrap_or_default();
    let exe_dir = exe.parent().unwrap_or(Path::new("."));

    // Check for templates next to binary first
    let beside_exe = exe_dir.join("templates");
    if beside_exe.exists() {
        return beside_exe;
    }

    // Development: look relative to CARGO_MANIFEST_DIR
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let dev_path = PathBuf::from(manifest_dir).join("templates");
        if dev_path.exists() {
            return dev_path;
        }
    }

    // CWD fallback (e.g. running from repo root)
    let cwd_path = PathBuf::from("templates");
    if cwd_path.exists() {
        return cwd_path;
    }

    // Extract embedded templates to cache directory
    extract_embedded_templates()
}

fn extract_embedded_templates() -> PathBuf {
    let version = env!("CARGO_PKG_VERSION");
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("fledge")
        .join(format!("templates-v{}", version));

    let should_extract = if cache_dir.exists() {
        // Re-extract if the binary is newer than the cache
        let binary_mtime = std::env::current_exe()
            .ok()
            .and_then(|p| p.metadata().ok())
            .and_then(|m| m.modified().ok());
        let cache_mtime = cache_dir.metadata().ok().and_then(|m| m.modified().ok());
        match (binary_mtime, cache_mtime) {
            (Some(bin), Some(cache)) => bin > cache,
            _ => false,
        }
    } else {
        true
    };

    if should_extract {
        let _ = std::fs::remove_dir_all(&cache_dir);
        if let Err(e) = extract_dir_recursive(&EMBEDDED_TEMPLATES, &cache_dir) {
            eprintln!("Warning: failed to extract embedded templates: {}", e);
        }
    }

    cache_dir
}

fn extract_dir_recursive(dir: &Dir, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target).with_context(|| format!("creating {}", target.display()))?;

    for file in dir.files() {
        let file_path = target.join(file.path().file_name().unwrap_or_default());
        std::fs::write(&file_path, file.contents())
            .with_context(|| format!("writing {}", file_path.display()))?;
    }

    for subdir in dir.dirs() {
        let subdir_name = subdir.path().file_name().unwrap_or_default();
        let subdir_target = target.join(subdir_name);
        extract_dir_recursive(subdir, &subdir_target)?;
    }

    Ok(())
}

fn load_templates_from_dir(dir: &Path, templates: &mut Vec<Template>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("template.toml");
        if !manifest_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?;
        let manifest: TemplateManifest = toml::from_str(&content)
            .with_context(|| format!("parsing {}", manifest_path.display()))?;

        templates.push(Template {
            name: manifest.template.name.clone(),
            description: manifest.template.description.clone(),
            path: path.to_path_buf(),
            manifest,
            source: None,
        });
    }
    Ok(())
}

pub fn render_template(
    template: &Template,
    target_dir: &Path,
    variables: &tera::Context,
) -> Result<Vec<PathBuf>> {
    let mut created_files = Vec::new();
    let template_dir = &template.path;
    let ignore_set: Vec<&str> = template
        .manifest
        .files
        .ignore
        .iter()
        .map(|s| s.as_str())
        .collect();

    for entry in WalkDir::new(template_dir).min_depth(1) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(template_dir)?;
        let rel_str = rel_path.to_string_lossy();

        // Skip ignored files
        if ignore_set.iter().any(|ig| matches_glob(ig, &rel_str)) {
            continue;
        }

        if entry.file_type().is_dir() {
            let dir_name = rel_str.to_string();
            let rendered_name = render_path_string(&dir_name, variables)?;
            let target = safe_join(target_dir, &rendered_name)?;
            std::fs::create_dir_all(&target)?;
            continue;
        }

        let rel_string = rel_str.to_string();

        // Check if file is a .tera file (always rendered, extension stripped)
        let (is_tera_ext, output_rel) = if rel_string.ends_with(".tera") {
            (true, rel_string.trim_end_matches(".tera").to_string())
        } else {
            (false, rel_string.clone())
        };

        // Render the path itself (handles {{ project_name_pascal }} in paths)
        let rendered_rel = render_path_string(&output_rel, variables)?;
        let target_path = safe_join(target_dir, &rendered_rel)?;

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Precedence: `.tera` extension wins (explicit render signal), then
        // `copy` (explicit verbatim signal), then `render` globs, then default
        // (copy as bytes). `ignore` was already short-circuited above.
        let force_copy = !is_tera_ext
            && template
                .manifest
                .files
                .copy
                .iter()
                .any(|g| matches_glob(g, &rel_string));

        let should_render = is_tera_ext
            || (!force_copy
                && template
                    .manifest
                    .files
                    .render
                    .iter()
                    .any(|g| matches_glob(g, &rel_string)));

        if should_render {
            let content = std::fs::read_to_string(entry.path())
                .with_context(|| format!("reading template file {}", entry.path().display()))?;
            let mut tera = Tera::default();
            tera.add_raw_template("__file__", &content)
                .with_context(|| format!("parsing template {}", rel_string))?;
            let rendered = tera
                .render("__file__", variables)
                .with_context(|| format!("rendering {}", rel_string))?;
            std::fs::write(&target_path, rendered)?;
        } else {
            std::fs::copy(entry.path(), &target_path)?;
        }

        created_files.push(PathBuf::from(&rendered_rel));
    }

    created_files.sort();
    Ok(created_files)
}

fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let joined = base.join(rel);
    for component in joined.strip_prefix(base).unwrap_or(&joined).components() {
        if matches!(component, Component::ParentDir) {
            bail!("path traversal rejected: {}", rel);
        }
    }
    Ok(joined)
}

fn render_path_string(path: &str, ctx: &tera::Context) -> Result<String> {
    if !path.contains("{{") {
        return Ok(path.to_string());
    }
    let mut tera = Tera::default();
    tera.add_raw_template("__path__", path)?;
    Ok(tera.render("__path__", ctx)?)
}

pub fn matches_glob_pub(pattern: &str, path: &str) -> bool {
    matches_glob(pattern, path)
}

fn matches_glob(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }
    let pattern = pattern.replace(".", r"\.");
    let pattern = pattern.replace("**/*", "\x00ANY\x00");
    let pattern = pattern.replace("**", "\x00ANY\x00");
    let pattern = pattern.replace('*', "[^/]*");
    let pattern = pattern.replace("\x00ANY\x00", ".*");
    let re = regex_lite::Regex::new(&format!("^{pattern}$"));
    match re {
        Ok(r) => r.is_match(path),
        Err(_) => pattern == path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- matches_glob tests ---

    #[test]
    fn glob_exact_match() {
        assert!(matches_glob("template.toml", "template.toml"));
    }

    #[test]
    fn glob_exact_no_match() {
        assert!(!matches_glob("template.toml", "other.toml"));
    }

    #[test]
    fn glob_star_matches_within_segment() {
        assert!(matches_glob("*.rs", "main.rs"));
        assert!(matches_glob("*.rs", "lib.rs"));
        assert!(!matches_glob("*.rs", "src/main.rs"));
    }

    #[test]
    fn glob_double_star_matches_any_path() {
        assert!(matches_glob("**/*.rs", "src/main.rs"));
        assert!(matches_glob("**/*.rs", "src/nested/deep/main.rs"));
        assert!(matches_glob("**/*.rs", "main.rs"));
    }

    #[test]
    fn glob_double_star_matches_dirs() {
        assert!(matches_glob("**/*.yml", ".github/workflows/ci.yml"));
    }

    #[test]
    fn glob_dots_escaped() {
        assert!(matches_glob("*.rs", "main.rs"));
        assert!(!matches_glob("*.rs", "mainXrs"));
    }

    #[test]
    fn glob_star_does_not_cross_dirs() {
        assert!(!matches_glob("*.rs", "src/main.rs"));
    }

    // --- render_path_string tests ---

    #[test]
    fn render_path_no_vars_passthrough() {
        let ctx = tera::Context::new();
        assert_eq!(
            render_path_string("src/main.rs", &ctx).unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn render_path_with_variable() {
        let mut ctx = tera::Context::new();
        ctx.insert("project_name", "my-app");
        assert_eq!(
            render_path_string("src/{{ project_name }}/lib.rs", &ctx).unwrap(),
            "src/my-app/lib.rs"
        );
    }

    #[test]
    fn render_path_with_pascal_case() {
        let mut ctx = tera::Context::new();
        ctx.insert("project_name_pascal", "MyApp");
        assert_eq!(
            render_path_string("{{ project_name_pascal }}.swift", &ctx).unwrap(),
            "MyApp.swift"
        );
    }

    #[test]
    fn render_path_missing_variable_errors() {
        let ctx = tera::Context::new();
        let result = render_path_string("{{ missing_var }}", &ctx);
        assert!(result.is_err());
    }

    // --- Template manifest parsing ---

    #[test]
    fn parse_minimal_manifest() {
        let toml_str = r#"
[template]
name = "test"
description = "A test template"

[files]
render = ["**/*.rs"]
"#;
        let manifest: TemplateManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.template.name, "test");
        assert_eq!(manifest.template.description, "A test template");
        assert_eq!(manifest.files.render, vec!["**/*.rs"]);
        assert!(manifest.files.ignore.is_empty());
        assert!(manifest.prompts.is_empty());
        assert!(manifest.hooks.post_create.is_empty());
    }

    #[test]
    fn parse_manifest_with_hooks() {
        let toml_str = r#"
[template]
name = "test"
description = "A test template"

[hooks]
post_create = ["npm install", "echo done"]
"#;
        let manifest: TemplateManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.hooks.post_create.len(), 2);
        assert_eq!(manifest.hooks.post_create[0], "npm install");
    }

    #[test]
    fn parse_manifest_with_prompts() {
        let toml_str = r#"
[template]
name = "test"
description = "A test template"

[prompts.description]
message = "Project description"
default = "A {{ project_name }} project"
"#;
        let manifest: TemplateManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.prompts.len(), 1);
        let desc = manifest.prompts.get("description").unwrap();
        assert_eq!(desc.message, "Project description");
        assert_eq!(
            desc.default.as_deref(),
            Some("A {{ project_name }} project")
        );
    }

    #[test]
    fn parse_manifest_with_ignore_rules() {
        let toml_str = r#"
[template]
name = "test"
description = "Test"

[files]
render = ["**/*.rs"]
ignore = ["template.toml", "**/*.bak"]
"#;
        let manifest: TemplateManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.files.ignore, vec!["template.toml", "**/*.bak"]);
    }

    #[test]
    fn parse_manifest_with_copy_rules() {
        let toml_str = r#"
[template]
name = "test"
description = "Test"

[files]
render = ["**/*"]
copy = ["**/*.png", "**/*.ico"]
"#;
        let manifest: TemplateManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.files.copy, vec!["**/*.png", "**/*.ico"]);
    }

    #[test]
    fn prompts_iterate_in_deterministic_order() {
        // Locks the templates v1 contract: prompt iteration is alphabetical
        // by key, so multi-prompt templates ask questions in a stable order.
        let toml_str = r#"
[template]
name = "test"
description = "Test"

[prompts.zebra]
message = "Zebra?"

[prompts.alpha]
message = "Alpha?"

[prompts.mango]
message = "Mango?"
"#;
        let manifest: TemplateManifest = toml::from_str(toml_str).unwrap();
        let keys: Vec<&str> = manifest.prompts.keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["alpha", "mango", "zebra"]);
    }

    #[test]
    fn parse_invalid_manifest_errors() {
        let result: Result<TemplateManifest, _> = toml::from_str("not valid toml");
        assert!(result.is_err());
    }

    #[test]
    fn parse_manifest_missing_required_fields_errors() {
        let result: Result<TemplateManifest, _> = toml::from_str("[template]\nname = \"x\"");
        assert!(result.is_err());
    }

    // --- discover_templates tests ---

    #[test]
    fn discover_from_empty_extra_paths() {
        let templates = discover_templates(&[]).unwrap();
        // Should at least find built-in templates during dev
        assert!(!templates.is_empty());
    }

    #[test]
    fn discover_ignores_nonexistent_extra_paths() {
        let templates = discover_templates(&[PathBuf::from("/nonexistent/path")]).unwrap();
        assert!(!templates.is_empty()); // built-ins still found
    }

    #[test]
    fn discover_templates_sorted_alphabetically() {
        let templates = discover_templates(&[]).unwrap();
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn discover_from_extra_dir() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("my-tpl");
        fs::create_dir(&tpl_dir).unwrap();
        fs::write(
            tpl_dir.join("template.toml"),
            r#"
[template]
name = "my-tpl"
description = "Custom template"

[files]
render = ["**/*.txt"]
"#,
        )
        .unwrap();
        fs::write(tpl_dir.join("hello.txt"), "Hello {{ project_name }}").unwrap();

        let templates = discover_templates(&[tmp.path().to_path_buf()]).unwrap();
        assert!(templates.iter().any(|t| t.name == "my-tpl"));
    }

    #[test]
    fn discover_skips_dirs_without_manifest() {
        let tmp = TempDir::new().unwrap();
        let no_manifest = tmp.path().join("no-manifest");
        fs::create_dir(&no_manifest).unwrap();
        fs::write(no_manifest.join("file.txt"), "hello").unwrap();

        let templates = discover_templates(&[tmp.path().to_path_buf()]).unwrap();
        assert!(!templates.iter().any(|t| t.name == "no-manifest"));
    }

    // --- render_template tests ---

    fn make_test_template(dir: &Path, name: &str, files: &[(&str, &str)], manifest: &str) {
        let tpl_dir = dir.join(name);
        fs::create_dir_all(&tpl_dir).unwrap();
        fs::write(tpl_dir.join("template.toml"), manifest).unwrap();
        for (path, content) in files {
            let file_path = tpl_dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }
    }

    fn load_test_template(dir: &Path, name: &str) -> Template {
        let mut templates = Vec::new();
        load_templates_from_dir(dir, &mut templates).unwrap();
        templates.into_iter().find(|t| t.name == name).unwrap()
    }

    #[test]
    fn render_tera_file_strips_extension_and_renders() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "test-tpl",
            &[("README.md.tera", "# {{ project_name }}\nBy {{ author }}")],
            r#"
[template]
name = "test-tpl"
description = "Test"

[files]
render = []
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "test-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let mut ctx = tera::Context::new();
        ctx.insert("project_name", "my-app");
        ctx.insert("author", "Leif");

        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("README.md")));
        assert!(!files.iter().any(|f| f.to_string_lossy().contains(".tera")));

        let content = fs::read_to_string(target.join("README.md")).unwrap();
        assert_eq!(content, "# my-app\nBy Leif");
    }

    #[test]
    fn render_glob_matched_files() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "glob-tpl",
            &[("src/main.rs", "fn main() { // {{ project_name }} }")],
            r#"
[template]
name = "glob-tpl"
description = "Test"

[files]
render = ["**/*.rs"]
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "glob-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let mut ctx = tera::Context::new();
        ctx.insert("project_name", "cool-app");

        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("src/main.rs")));

        let content = fs::read_to_string(target.join("src/main.rs")).unwrap();
        assert!(content.contains("cool-app"));
    }

    #[test]
    fn render_copies_non_matched_files() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "copy-tpl",
            &[("image.png", "binary-data-here")],
            r#"
[template]
name = "copy-tpl"
description = "Test"

[files]
render = ["**/*.rs"]
copy = ["**/*.png"]
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "copy-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let ctx = tera::Context::new();
        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("image.png")));

        let content = fs::read_to_string(target.join("image.png")).unwrap();
        assert_eq!(content, "binary-data-here");
    }

    #[test]
    fn copy_glob_overrides_render_glob() {
        // Locks templates v1 contract: a file matching both render and copy is
        // copied verbatim. Without this, a broad `render = ["**/*"]` would
        // corrupt binary assets even when the author explicitly listed them
        // under `copy`.
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        let raw_bytes = "{{ this_is_not_a_template }}";
        make_test_template(
            &tpl_dir,
            "force-copy",
            &[("logo.png", raw_bytes)],
            r#"
[template]
name = "force-copy"
description = "Test"

[files]
render = ["**/*"]
copy = ["**/*.png"]
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "force-copy");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let ctx = tera::Context::new();
        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("logo.png")));

        let content = fs::read_to_string(target.join("logo.png")).unwrap();
        assert_eq!(
            content, raw_bytes,
            "copy glob should bypass Tera even when render glob matches"
        );
    }

    #[test]
    fn tera_extension_wins_over_copy_glob() {
        // `.tera` is the explicit "render this" signal — it overrides copy.
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "tera-wins",
            &[("config.toml.tera", "name = \"{{ project_name }}\"")],
            r#"
[template]
name = "tera-wins"
description = "Test"

[files]
copy = ["**/*"]
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "tera-wins");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let mut ctx = tera::Context::new();
        ctx.insert("project_name", "rendered");

        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("config.toml")));

        let content = fs::read_to_string(target.join("config.toml")).unwrap();
        assert_eq!(content, "name = \"rendered\"");
    }

    #[test]
    fn render_ignores_matching_files() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "ignore-tpl",
            &[("keep.txt", "visible"), ("secret.bak", "hidden")],
            r#"
[template]
name = "ignore-tpl"
description = "Test"

[files]
render = []
ignore = ["template.toml", "**/*.bak"]
"#,
        );

        let template = load_test_template(&tpl_dir, "ignore-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let ctx = tera::Context::new();
        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("keep.txt")));
        assert!(!files.iter().any(|f| f.to_string_lossy().contains("secret")));
        assert!(!target.join("secret.bak").exists());
    }

    #[test]
    fn render_path_variables_in_dirs() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "path-tpl",
            &[("{{ project_name_pascal }}/mod.rs", "// module")],
            r#"
[template]
name = "path-tpl"
description = "Test"

[files]
render = []
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "path-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let mut ctx = tera::Context::new();
        ctx.insert("project_name_pascal", "MyApp");

        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("MyApp/mod.rs")));
        assert!(target.join("MyApp/mod.rs").exists());
    }

    #[test]
    fn render_output_is_sorted() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "sort-tpl",
            &[
                ("z_file.txt", "z"),
                ("a_file.txt", "a"),
                ("m_file.txt", "m"),
            ],
            r#"
[template]
name = "sort-tpl"
description = "Test"

[files]
render = []
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "sort-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let ctx = tera::Context::new();
        let files = render_template(&template, &target, &ctx).unwrap();
        let file_strs: Vec<String> = files
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        let mut sorted = file_strs.clone();
        sorted.sort();
        assert_eq!(file_strs, sorted);
    }

    #[test]
    fn discover_builtin_templates_finds_starters() {
        let templates = discover_templates(&[]).unwrap();
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"rust-cli"), "missing rust-cli");
        assert!(names.contains(&"ts-bun"), "missing ts-bun");
        assert!(names.contains(&"python-cli"), "missing python-cli");
        assert!(names.contains(&"go-cli"), "missing go-cli");
        assert!(names.contains(&"ts-node"), "missing ts-node");
        assert!(names.contains(&"static-site"), "missing static-site");
        assert!(names.contains(&"kotlin-kmp"), "missing kotlin-kmp");
        assert!(
            names.contains(&"kotlin-ktor-api"),
            "missing kotlin-ktor-api"
        );
        assert!(names.contains(&"fledge-plugin"), "missing fledge-plugin");
        assert!(names.contains(&"corvid-stack"), "missing corvid-stack");
        assert_eq!(names.len(), 10, "expected exactly 10 built-in templates");
    }

    #[test]
    fn render_template_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "nested-tpl",
            &[("src/deeply/nested/file.txt", "content")],
            r#"
[template]
name = "nested-tpl"
description = "Test"

[files]
render = []
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "nested-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let ctx = tera::Context::new();
        let files = render_template(&template, &target, &ctx).unwrap();
        assert!(files.contains(&PathBuf::from("src/deeply/nested/file.txt")));
        assert!(target.join("src/deeply/nested/file.txt").exists());
    }

    #[test]
    fn render_template_with_missing_var_errors() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir(&tpl_dir).unwrap();

        make_test_template(
            &tpl_dir,
            "err-tpl",
            &[("file.txt.tera", "Hello {{ nonexistent }}")],
            r#"
[template]
name = "err-tpl"
description = "Test"

[files]
render = []
ignore = ["template.toml"]
"#,
        );

        let template = load_test_template(&tpl_dir, "err-tpl");
        let target = tmp.path().join("output");
        fs::create_dir(&target).unwrap();

        let ctx = tera::Context::new();
        let result = render_template(&template, &target, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn check_requirements_finds_sh() {
        let (found, missing) = check_requirements(&["sh".to_string()]);
        assert_eq!(found, vec!["sh"]);
        assert!(missing.is_empty());
    }

    #[test]
    fn check_requirements_reports_missing() {
        let (found, missing) = check_requirements(&["fledge_nonexistent_xyz".to_string()]);
        assert!(found.is_empty());
        assert_eq!(missing, vec!["fledge_nonexistent_xyz"]);
    }

    #[test]
    fn check_requirements_mixed() {
        let (found, missing) =
            check_requirements(&["sh".to_string(), "fledge_nonexistent_xyz".to_string()]);
        assert_eq!(found, vec!["sh"]);
        assert_eq!(missing, vec!["fledge_nonexistent_xyz"]);
    }

    #[test]
    fn check_requirements_empty_input() {
        let (found, missing) = check_requirements(&[]);
        assert!(found.is_empty());
        assert!(missing.is_empty());
    }

    #[test]
    fn check_requirements_rejects_dash_prefix() {
        let (found, missing) = check_requirements(&["--version".to_string()]);
        assert!(found.is_empty());
        assert_eq!(missing, vec!["--version"]);
    }

    #[test]
    fn check_requirements_rejects_invalid_chars() {
        let (found, missing) = check_requirements(&[
            "".to_string(),
            "tool/path".to_string(),
            "tool\\path".to_string(),
            "tool\0path".to_string(),
        ]);
        assert!(found.is_empty());
        assert_eq!(missing, vec!["", "tool/path", "tool\\path", "tool\0path"]);
    }

    #[test]
    fn safe_join_rejects_traversal() {
        let base = Path::new("/tmp/project");
        assert!(safe_join(base, "../etc/passwd").is_err());
        assert!(safe_join(base, "src/../../../etc/passwd").is_err());
    }

    #[test]
    fn safe_join_allows_normal_paths() {
        let base = Path::new("/tmp/project");
        assert!(safe_join(base, "src/main.rs").is_ok());
        assert!(safe_join(base, "nested/dir/file.txt").is_ok());
    }
}
