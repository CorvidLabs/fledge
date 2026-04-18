use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tera::Tera;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
pub struct TemplateManifest {
    pub template: TemplateInfo,
    #[serde(default)]
    pub prompts: HashMap<String, PromptDef>,
    #[serde(default)]
    pub files: FileRules,
}

#[derive(Debug, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub min_fledge_version: Option<String>,
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
    #[serde(default)]
    #[allow(dead_code)]
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

    // Fallback: current directory
    PathBuf::from("templates")
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
            let target = target_dir.join(&rendered_name);
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
        let target_path = target_dir.join(&rendered_rel);

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let should_render = is_tera_ext
            || template
                .manifest
                .files
                .render
                .iter()
                .any(|g| matches_glob(g, &rel_string));

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

fn render_path_string(path: &str, ctx: &tera::Context) -> Result<String> {
    if !path.contains("{{") {
        return Ok(path.to_string());
    }
    let mut tera = Tera::default();
    tera.add_raw_template("__path__", path)?;
    Ok(tera.render("__path__", ctx)?)
}

fn matches_glob(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }
    // Simple glob: ** matches any path, * matches within a segment
    let pattern = pattern.replace(".", r"\.");
    let pattern = pattern.replace("**/*", ".*");
    let pattern = pattern.replace("**", ".*");
    let pattern = pattern.replace('*', "[^/]*");
    let re = regex_lite::Regex::new(&format!("^{pattern}$"));
    match re {
        Ok(r) => r.is_match(path),
        Err(_) => pattern == path,
    }
}
