use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) mod commands;
pub(crate) mod parse;
#[cfg(test)]
mod tests;
pub(crate) mod validation;

// ── Constants ────────────────────────────────────────────────────────────────

pub(crate) const COMPANION_FILES: &[&str] =
    &["requirements.md", "tasks.md", "context.md", "testing.md"];

/// Per-command JSON schema versions for `spec` subcommands. See lanes.rs for
/// rationale.
pub(crate) const SPEC_CHECK_SCHEMA: u32 = 1;
pub(crate) const SPEC_LIST_SCHEMA: u32 = 1;
pub(crate) const SPEC_SHOW_SCHEMA: u32 = 1;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct SpecSyncConfig {
    pub(crate) specs_dir: Option<String>,
    #[serde(default)]
    pub(crate) required_sections: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpecFrontmatter {
    pub module: String,
    pub version: u32,
    pub status: String,
    #[serde(default)]
    pub files: Vec<String>,
}

/// A compact entry suitable for prompt-context indexes.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub name: String,
    pub version: u32,
    pub status: String,
    pub purpose: Option<String>,
    pub files: Vec<String>,
    /// Absolute path to the `.spec.md` file on disk. Tracked so callers can
    /// resolve specs that live in shared directories (e.g.
    /// `specs/plugin/plugin-protocol.spec.md` declaring `module: plugin-protocol`).
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum SpecAction {
    Check { strict: bool, json: bool },
    Init,
    New { name: String },
    List { json: bool },
    Show { name: String, json: bool },
}

// ── Config / root helpers ────────────────────────────────────────────────────

pub(crate) fn load_config(project_root: &Path) -> Result<SpecSyncConfig> {
    let config_path = project_root.join(".specsync/config.toml");
    if !config_path.exists() {
        bail!(
            "No .specsync/config.toml found. Run {} to initialize.",
            console::style("fledge spec init").cyan()
        );
    }
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: SpecSyncConfig =
        toml::from_str(&content).with_context(|| "Failed to parse .specsync/config.toml")?;
    Ok(config)
}

pub(crate) fn find_project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub(crate) fn specs_dir_from_config(root: &Path) -> Result<PathBuf> {
    let config = load_config(root)?;
    Ok(root.join(config.specs_dir.as_deref().unwrap_or("specs")))
}

// ── File helpers ─────────────────────────────────────────────────────────────

pub(crate) fn find_spec_files(specs_dir: &Path) -> Vec<PathBuf> {
    let mut spec_paths = Vec::new();
    for entry in WalkDir::new(specs_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".spec.md") {
                spec_paths.push(path.to_path_buf());
            }
        }
    }
    spec_paths
}

pub(crate) fn classify_companions(spec_dir: &Path) -> (Vec<String>, Vec<String>) {
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for companion in COMPANION_FILES {
        if spec_dir.join(companion).exists() {
            present.push((*companion).to_string());
        } else {
            missing.push((*companion).to_string());
        }
    }
    (present, missing)
}

pub(crate) fn validate_module_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Module name cannot be empty");
    }
    if name.contains('\\') {
        bail!("Invalid module name '{name}': may not contain '\\'; use '/' for nested specs");
    }
    if name.starts_with('/') || name.ends_with('/') {
        bail!("Invalid module name '{name}': may not start or end with '/'");
    }
    for segment in name.split('/') {
        if segment.is_empty() {
            bail!("Invalid module name '{name}': empty path segment");
        }
        if segment == "." || segment == ".." {
            bail!("Invalid module name '{name}': segment '{segment}' is not allowed");
        }
    }
    Ok(())
}

/// Leaf segment of a (possibly nested) module name. For `game/board` returns `board`;
/// for a flat name returns the name itself.
pub(crate) fn module_leaf(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

// ── Utility ──────────────────────────────────────────────────────────────────

pub(crate) fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── run() dispatcher ─────────────────────────────────────────────────────────

pub fn run(action: SpecAction) -> Result<()> {
    let root = find_project_root();
    match action {
        SpecAction::Check { strict, json } => commands::check(&root, strict, json),
        SpecAction::Init => commands::init(&root),
        SpecAction::New { name } => commands::new_spec(&root, &name),
        SpecAction::List { json } => commands::list_specs(&root, json),
        SpecAction::Show { name, json } => commands::show_spec(&root, &name, json),
    }
}

// ── Public index APIs ────────────────────────────────────────────────────────

/// Collect a compact, sorted index of every spec in the project.
/// Intended for feeding into LLM prompts or other machine-readable consumers.
pub fn collect_index(root: &Path) -> Result<Vec<IndexEntry>> {
    let specs_dir = specs_dir_from_config(root)?;
    if !specs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for path in find_spec_files(&specs_dir) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok((fm, body)) = parse::parse_frontmatter(&content) else {
            continue;
        };
        entries.push(IndexEntry {
            name: fm.module,
            version: fm.version,
            status: fm.status,
            purpose: parse::extract_purpose(&body),
            files: fm.files,
            path,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Render an index as a compact markdown block suitable for prompt injection.
pub fn render_index_markdown(entries: &[IndexEntry]) -> String {
    let mut out = String::from("## Available specs\n\n");
    out.push_str(
        "This project documents each module in `specs/<name>/`. \
         Run `fledge spec show <name>` for the full detail.\n\n",
    );
    for entry in entries {
        let purpose = entry
            .purpose
            .as_deref()
            .unwrap_or("(no purpose documented)");
        let files = if entry.files.is_empty() {
            String::new()
        } else {
            format!(" — {}", entry.files.join(", "))
        };
        out.push_str(&format!(
            "- **{}** v{} ({}){} — {}\n",
            entry.name, entry.version, entry.status, files, purpose,
        ));
    }
    out
}

/// Return every module name that has a `specs/<name>/<name>.spec.md` file.
pub fn all_module_names(root: &Path) -> Result<Vec<String>> {
    Ok(collect_index(root)?.into_iter().map(|e| e.name).collect())
}

/// Return the module names whose `files:` frontmatter matches any of the given
/// paths, or whose `<specs_dir>/<name>/` directory contains any of them.
///
/// Used by `fledge review` to automatically include the right spec context
/// when reviewing a diff. Silent on specs that fail to parse — a broken
/// spec should never block a review.
///
/// Honors the `specs_dir` key from `.specsync/config.toml`, so projects that
/// put specs under e.g. `docs/specs/` match correctly.
pub fn specs_for_changed_files(root: &Path, changed_files: &[String]) -> Result<Vec<String>> {
    let index = collect_index(root)?;

    let mut matched = Vec::new();
    for entry in &index {
        let files_match = entry
            .files
            .iter()
            .any(|f| changed_files.iter().any(|c| c == f));
        // Use the spec file's actual parent directory rather than assuming
        // `<specs_dir>/<name>/`. Sub-specs that share a directory with another
        // module (e.g. `specs/plugin/plugin-protocol.spec.md`) resolve correctly,
        // and so do specs at the top level of a custom `specs_dir`.
        let dir_match = entry
            .path
            .parent()
            .and_then(|parent| parent.strip_prefix(root).ok())
            .map(|rel_parent| {
                let mut prefix = rel_parent.to_string_lossy().replace('\\', "/");
                if !prefix.ends_with('/') {
                    prefix.push('/');
                }
                changed_files.iter().any(|c| c.starts_with(&prefix))
            })
            .unwrap_or(false);
        if files_match || dir_match {
            matched.push(entry.name.clone());
        }
    }
    matched.sort();
    matched.dedup();
    Ok(matched)
}

/// Load the full spec bundle for a single module: its `.spec.md` plus whichever
/// companion files exist, concatenated as a single markdown block with headers.
pub fn load_module_bundle(root: &Path, name: &str) -> Result<String> {
    validate_module_name(name)?;

    // Resolve via the index so sub-specs (e.g. `specs/plugin/plugin-protocol.spec.md`
    // declaring `module: plugin-protocol`) load from their real on-disk location.
    let entry = collect_index(root)
        .ok()
        .and_then(|index| index.into_iter().find(|e| e.name == name));

    let spec_path = match entry {
        Some(e) => e.path,
        None => {
            // Preserve the prior error path: report the conventional location.
            let specs_dir = specs_dir_from_config(root)?;
            let fallback = specs_dir.join(name).join(format!("{name}.spec.md"));
            bail!(
                "No spec found for '{}' (looked at {})",
                name,
                fallback.display()
            );
        }
    };

    let module_dir = spec_path.parent().unwrap_or(&spec_path).to_path_buf();
    let spec_filename = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{name}.spec.md"));

    let mut bundle = String::new();
    bundle.push_str(&format!("## Spec bundle: {name}\n\n"));

    let spec_content = fs::read_to_string(&spec_path)
        .with_context(|| format!("reading {}", spec_path.display()))?;
    bundle.push_str(&format!("### `{spec_filename}`\n\n"));
    bundle.push_str(spec_content.trim_end());
    bundle.push_str("\n\n");

    for companion in COMPANION_FILES {
        let companion_path = module_dir.join(companion);
        if !companion_path.exists() {
            continue;
        }
        let companion_content = fs::read_to_string(&companion_path)
            .with_context(|| format!("reading {}", companion_path.display()))?;
        bundle.push_str(&format!("### `{companion}`\n\n"));
        bundle.push_str(companion_content.trim_end());
        bundle.push_str("\n\n");
    }

    Ok(bundle)
}
