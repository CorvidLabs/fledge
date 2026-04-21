use anyhow::{Context, Result, bail};
use console::style;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::templates;

pub struct UpdateOptions {
    pub dry_run: bool,
    pub refresh: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub source: SourceInfo,
    pub variables: BTreeMap<String, String>,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub fledge_version: String,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
}

#[derive(Debug)]
pub enum UpdateAction {
    Add(PathBuf),
    Update(PathBuf),
    Skip(PathBuf),
    Remove(PathBuf),
}

pub fn run(opts: UpdateOptions) -> Result<()> {
    let project_dir = std::env::current_dir().context("getting current directory")?;
    let meta_path = project_dir.join(".fledge.toml");

    if !meta_path.exists() {
        bail!("No .fledge.toml found. Was this project created with fledge?");
    }

    let meta_content = std::fs::read_to_string(&meta_path).context("reading .fledge.toml")?;
    let meta: ProjectMeta = toml::from_str(&meta_content).context("parsing .fledge.toml")?;

    println!(
        "{} Updating project from template: {}",
        style("*").cyan().bold(),
        style(&meta.source.template).green()
    );

    let config = Config::load().context("loading config")?;
    let template = resolve_source_template(&meta, &config, opts.refresh)?;

    let variables = meta_to_tera_context(&meta);

    let new_files = render_to_memory(&template, &variables)?;

    let actions = compute_actions(&meta, &new_files, &project_dir)?;

    let adds: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, UpdateAction::Add(_)))
        .collect();
    let updates: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, UpdateAction::Update(_)))
        .collect();
    let skips: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, UpdateAction::Skip(_)))
        .collect();
    let removes: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, UpdateAction::Remove(_)))
        .collect();

    if adds.is_empty() && updates.is_empty() && removes.is_empty() {
        println!("{} Already up to date.", style("✓").green().bold());
        return Ok(());
    }

    if opts.dry_run {
        print_dry_run(&adds, &updates, &skips, &removes);
        return Ok(());
    }

    let mut updated_files = meta.files.clone();

    for action in &actions {
        match action {
            UpdateAction::Add(path) => {
                let content = &new_files[&path.to_string_lossy().to_string()];
                let target = project_dir.join(path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, content)?;
                updated_files.insert(
                    path.to_string_lossy().to_string(),
                    compute_file_hash(content),
                );
                println!("  {} {}", style("+ add").green(), path.display());
            }
            UpdateAction::Update(path) => {
                let content = &new_files[&path.to_string_lossy().to_string()];
                let target = project_dir.join(path);
                std::fs::write(&target, content)?;
                updated_files.insert(
                    path.to_string_lossy().to_string(),
                    compute_file_hash(content),
                );
                println!("  {} {}", style("~ update").yellow(), path.display());
            }
            UpdateAction::Skip(path) => {
                println!(
                    "  {} {} (modified by user)",
                    style("- skip").dim(),
                    path.display()
                );
            }
            UpdateAction::Remove(path) => {
                println!(
                    "  {} {} (removed from template, keeping)",
                    style("! warn").yellow().bold(),
                    path.display()
                );
            }
        }
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let new_version = template.manifest.template.version.clone();
    let updated_meta = ProjectMeta {
        source: SourceInfo {
            version: new_version.or(meta.source.version),
            updated: Some(today),
            ..meta.source
        },
        variables: meta.variables,
        files: updated_files,
    };

    let meta_toml = toml::to_string_pretty(&updated_meta).context("serializing .fledge.toml")?;
    std::fs::write(&meta_path, meta_toml).context("writing .fledge.toml")?;

    let change_count = adds.len() + updates.len();
    println!();
    println!(
        "{} Updated {} file{}.",
        style("✓").green().bold(),
        change_count,
        if change_count == 1 { "" } else { "s" }
    );
    if !skips.is_empty() {
        println!(
            "  {} file{} skipped (user-modified).",
            skips.len(),
            if skips.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

fn resolve_source_template(
    meta: &ProjectMeta,
    config: &Config,
    refresh: bool,
) -> Result<templates::Template> {
    if let Some(ref remote_ref) = meta.source.remote {
        if refresh {
            let (owner, repo, _, _) = crate::remote::parse_remote_ref(remote_ref);
            crate::remote::clear_cache(owner, repo)?;
        }

        let (owner, repo, subpath, git_ref) = crate::remote::parse_remote_ref(remote_ref);
        let ref_override = meta.source.git_ref.as_deref().or(git_ref);
        let token = config.github_token();
        let template_dir = crate::remote::resolve_template_dir(
            owner,
            repo,
            subpath,
            token.as_deref(),
            ref_override,
        )?;

        let manifest_path = template_dir.join("template.toml");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)?;
            let manifest: templates::TemplateManifest = toml::from_str(&content)?;
            return Ok(templates::Template {
                name: manifest.template.name.clone(),
                description: manifest.template.description.clone(),
                path: template_dir,
                manifest,
            });
        }

        let mut found = Vec::new();
        templates::load_templates_from_dir_pub(&template_dir, &mut found)?;
        found
            .into_iter()
            .find(|t| t.name == meta.source.template)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Template '{}' not found in remote {}",
                    meta.source.template,
                    remote_ref
                )
            })
    } else {
        let extra_paths = config.extra_template_paths();
        let token = config.github_token();
        let available = templates::discover_templates_with_repos(
            &extra_paths,
            config.template_repos(),
            token.as_deref(),
        )?;
        available
            .into_iter()
            .find(|t| t.name == meta.source.template)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Template '{}' not found. Is it still available?",
                    meta.source.template
                )
            })
    }
}

fn meta_to_tera_context(meta: &ProjectMeta) -> tera::Context {
    let mut ctx = tera::Context::new();
    for (key, value) in &meta.variables {
        ctx.insert(key, value);
    }
    ctx
}

fn render_to_memory(
    template: &templates::Template,
    variables: &tera::Context,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let template_dir = &template.path;
    let ignore_set: Vec<&str> = template
        .manifest
        .files
        .ignore
        .iter()
        .map(|s| s.as_str())
        .collect();

    for entry in walkdir::WalkDir::new(template_dir).min_depth(1) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(template_dir)?;
        let rel_str = rel_path.to_string_lossy();

        if ignore_set
            .iter()
            .any(|ig| templates::matches_glob_pub(ig, &rel_str))
        {
            continue;
        }

        if entry.file_type().is_dir() {
            continue;
        }

        let rel_string = rel_str.to_string();
        let (is_tera_ext, output_rel) = if rel_string.ends_with(".tera") {
            (true, rel_string.trim_end_matches(".tera").to_string())
        } else {
            (false, rel_string.clone())
        };

        let rendered_rel = render_path_string(&output_rel, variables)?;

        let should_render = is_tera_ext
            || template
                .manifest
                .files
                .render
                .iter()
                .any(|g| templates::matches_glob_pub(g, &rel_string));

        let content = if should_render {
            let text = std::fs::read_to_string(entry.path())
                .with_context(|| format!("reading template file {}", entry.path().display()))?;
            let mut tera = tera::Tera::default();
            tera.add_raw_template("__file__", &text)
                .with_context(|| format!("parsing template {}", rel_string))?;
            let rendered = tera
                .render("__file__", variables)
                .with_context(|| format!("rendering {}", rel_string))?;
            rendered.into_bytes()
        } else {
            std::fs::read(entry.path())
                .with_context(|| format!("reading {}", entry.path().display()))?
        };

        files.insert(rendered_rel, content);
    }

    Ok(files)
}

fn render_path_string(path: &str, ctx: &tera::Context) -> Result<String> {
    if !path.contains("{{") {
        return Ok(path.to_string());
    }
    let mut tera = tera::Tera::default();
    tera.add_raw_template("__path__", path)?;
    Ok(tera.render("__path__", ctx)?)
}

fn compute_actions(
    meta: &ProjectMeta,
    new_files: &BTreeMap<String, Vec<u8>>,
    project_dir: &Path,
) -> Result<Vec<UpdateAction>> {
    let mut actions = Vec::new();

    for (rel_path, new_content) in new_files {
        let path = PathBuf::from(rel_path);
        let full_path = project_dir.join(rel_path);
        let new_hash = compute_file_hash(new_content);

        if !full_path.exists() {
            actions.push(UpdateAction::Add(path));
            continue;
        }

        let old_hash = meta.files.get(rel_path);
        let current_content = std::fs::read(&full_path)
            .with_context(|| format!("reading {}", full_path.display()))?;
        let current_hash = compute_file_hash(&current_content);

        if current_hash == new_hash {
            continue;
        }

        match old_hash {
            Some(orig) if *orig == current_hash => {
                actions.push(UpdateAction::Update(path));
            }
            Some(_) | None => {
                actions.push(UpdateAction::Skip(path));
            }
        }
    }

    for rel_path in meta.files.keys() {
        if !new_files.contains_key(rel_path) {
            let full_path = project_dir.join(rel_path);
            if full_path.exists() {
                actions.push(UpdateAction::Remove(PathBuf::from(rel_path)));
            }
        }
    }

    actions.sort_by(|a, b| action_path(a).cmp(action_path(b)));
    Ok(actions)
}

fn action_path(action: &UpdateAction) -> &Path {
    match action {
        UpdateAction::Add(p)
        | UpdateAction::Update(p)
        | UpdateAction::Skip(p)
        | UpdateAction::Remove(p) => p,
    }
}

pub fn compute_file_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub fn write_project_meta(
    project_dir: &Path,
    template_name: &str,
    remote_ref: Option<&str>,
    git_ref: Option<&str>,
    template_version: Option<&str>,
    variables: &tera::Context,
    created_files: &[PathBuf],
) -> Result<()> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut var_map = BTreeMap::new();
    if let Some(obj) = variables.clone().into_json().as_object() {
        for (key, value) in obj {
            if let Some(s) = value.as_str() {
                var_map.insert(key.clone(), s.to_string());
            }
        }
    }

    let mut file_hashes = BTreeMap::new();
    for file in created_files {
        let full_path = project_dir.join(file);
        if full_path.exists() && full_path.is_file() {
            let content = std::fs::read(&full_path)
                .with_context(|| format!("reading {} for hash", full_path.display()))?;
            file_hashes.insert(
                file.to_string_lossy().to_string(),
                compute_file_hash(&content),
            );
        }
    }

    let meta = ProjectMeta {
        source: SourceInfo {
            template: template_name.to_string(),
            remote: remote_ref.map(|s| s.to_string()),
            git_ref: git_ref.map(|s| s.to_string()),
            version: template_version.map(|s| s.to_string()),
            fledge_version: env!("CARGO_PKG_VERSION").to_string(),
            created: today,
            updated: None,
        },
        variables: var_map,
        files: file_hashes,
    };

    let toml_str = toml::to_string_pretty(&meta).context("serializing .fledge.toml")?;
    let meta_path = project_dir.join(".fledge.toml");
    std::fs::write(&meta_path, &toml_str).context("writing .fledge.toml")?;

    Ok(())
}

fn print_dry_run(
    adds: &[&UpdateAction],
    updates: &[&UpdateAction],
    skips: &[&UpdateAction],
    removes: &[&UpdateAction],
) {
    println!();
    println!(
        "{} Dry run — nothing will be written",
        style("*").cyan().bold()
    );
    println!();

    for action in adds {
        if let UpdateAction::Add(p) = action {
            println!("  {} {}", style("+ add").green(), p.display());
        }
    }
    for action in updates {
        if let UpdateAction::Update(p) = action {
            println!("  {} {}", style("~ update").yellow(), p.display());
        }
    }
    for action in skips {
        if let UpdateAction::Skip(p) = action {
            println!(
                "  {} {} (modified by user)",
                style("- skip").dim(),
                p.display()
            );
        }
    }
    for action in removes {
        if let UpdateAction::Remove(p) = action {
            println!(
                "  {} {} (removed from template)",
                style("! warn").yellow().bold(),
                p.display()
            );
        }
    }

    println!();
    println!(
        "  {} to add, {} to update, {} skipped, {} warnings",
        adds.len(),
        updates.len(),
        skips.len(),
        removes.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn compute_hash_deterministic() {
        let hash1 = compute_file_hash(b"hello world");
        let hash2 = compute_file_hash(b"hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn compute_hash_different_for_different_content() {
        let hash1 = compute_file_hash(b"hello");
        let hash2 = compute_file_hash(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn compute_hash_format() {
        let hash = compute_file_hash(b"test");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn parse_project_meta() {
        let toml_str = r#"
[source]
template = "rust-cli"
fledge_version = "0.3.0"
created = "2026-04-19"

[variables]
project_name = "my-app"
author = "Leif"

[files]
"src/main.rs" = "abc123"
"Cargo.toml" = "def456"
"#;
        let meta: ProjectMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(meta.source.template, "rust-cli");
        assert_eq!(meta.source.fledge_version, "0.3.0");
        assert_eq!(meta.variables["project_name"], "my-app");
        assert_eq!(meta.variables["author"], "Leif");
        assert_eq!(meta.files.len(), 2);
    }

    #[test]
    fn parse_project_meta_with_remote() {
        let toml_str = r#"
[source]
template = "rust-cli"
remote = "CorvidLabs/fledge-templates"
git_ref = "v1.0"
version = "1.0.0"
fledge_version = "0.3.0"
created = "2026-04-19"
updated = "2026-04-20"

[variables]
project_name = "my-app"

[files]
"#;
        let meta: ProjectMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(
            meta.source.remote.as_deref(),
            Some("CorvidLabs/fledge-templates")
        );
        assert_eq!(meta.source.git_ref.as_deref(), Some("v1.0"));
        assert_eq!(meta.source.version.as_deref(), Some("1.0.0"));
        assert_eq!(meta.source.updated.as_deref(), Some("2026-04-20"));
    }

    #[test]
    fn parse_project_meta_minimal() {
        let toml_str = r#"
[source]
template = "rust-cli"
fledge_version = "0.3.0"
created = "2026-04-19"

[variables]

[files]
"#;
        let meta: ProjectMeta = toml::from_str(toml_str).unwrap();
        assert!(meta.source.remote.is_none());
        assert!(meta.source.git_ref.is_none());
        assert!(meta.variables.is_empty());
        assert!(meta.files.is_empty());
    }

    #[test]
    fn serialize_roundtrip() {
        let meta = ProjectMeta {
            source: SourceInfo {
                template: "rust-cli".to_string(),
                remote: None,
                git_ref: None,
                version: Some("1.0.0".to_string()),
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::from([
                ("project_name".to_string(), "my-app".to_string()),
                ("author".to_string(), "Leif".to_string()),
            ]),
            files: BTreeMap::from([("src/main.rs".to_string(), "abc123".to_string())]),
        };

        let serialized = toml::to_string_pretty(&meta).unwrap();
        let deserialized: ProjectMeta = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.source.template, "rust-cli");
        assert_eq!(deserialized.variables["project_name"], "my-app");
        assert_eq!(deserialized.files["src/main.rs"], "abc123");
    }

    #[test]
    fn meta_to_tera_context_populates_all_vars() {
        let meta = ProjectMeta {
            source: SourceInfo {
                template: "test".to_string(),
                remote: None,
                git_ref: None,
                version: None,
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::from([
                ("project_name".to_string(), "my-app".to_string()),
                ("author".to_string(), "Test".to_string()),
            ]),
            files: BTreeMap::new(),
        };

        let ctx = meta_to_tera_context(&meta);
        let json = ctx.into_json();
        assert_eq!(json["project_name"], "my-app");
        assert_eq!(json["author"], "Test");
    }

    #[test]
    fn compute_actions_new_file() {
        let meta = ProjectMeta {
            source: SourceInfo {
                template: "test".to_string(),
                remote: None,
                git_ref: None,
                version: None,
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::new(),
            files: BTreeMap::new(),
        };

        let tmp = TempDir::new().unwrap();
        let mut new_files = BTreeMap::new();
        new_files.insert("new-file.txt".to_string(), b"content".to_vec());

        let actions = compute_actions(&meta, &new_files, tmp.path()).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], UpdateAction::Add(p) if p == Path::new("new-file.txt")));
    }

    #[test]
    fn compute_actions_unmodified_file_updated() {
        let tmp = TempDir::new().unwrap();
        let old_content = b"old content";
        let new_content = b"new content";

        fs::write(tmp.path().join("file.txt"), old_content).unwrap();

        let meta = ProjectMeta {
            source: SourceInfo {
                template: "test".to_string(),
                remote: None,
                git_ref: None,
                version: None,
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::new(),
            files: BTreeMap::from([("file.txt".to_string(), compute_file_hash(old_content))]),
        };

        let mut new_files = BTreeMap::new();
        new_files.insert("file.txt".to_string(), new_content.to_vec());

        let actions = compute_actions(&meta, &new_files, tmp.path()).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], UpdateAction::Update(p) if p == Path::new("file.txt")));
    }

    #[test]
    fn compute_actions_user_modified_file_skipped() {
        let tmp = TempDir::new().unwrap();
        let original = b"original";
        let user_modified = b"user changed this";
        let template_new = b"template update";

        fs::write(tmp.path().join("file.txt"), user_modified).unwrap();

        let meta = ProjectMeta {
            source: SourceInfo {
                template: "test".to_string(),
                remote: None,
                git_ref: None,
                version: None,
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::new(),
            files: BTreeMap::from([("file.txt".to_string(), compute_file_hash(original))]),
        };

        let mut new_files = BTreeMap::new();
        new_files.insert("file.txt".to_string(), template_new.to_vec());

        let actions = compute_actions(&meta, &new_files, tmp.path()).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], UpdateAction::Skip(p) if p == Path::new("file.txt")));
    }

    #[test]
    fn compute_actions_removed_from_template() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("old-file.txt"), "content").unwrap();

        let meta = ProjectMeta {
            source: SourceInfo {
                template: "test".to_string(),
                remote: None,
                git_ref: None,
                version: None,
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::new(),
            files: BTreeMap::from([("old-file.txt".to_string(), compute_file_hash(b"content"))]),
        };

        let new_files = BTreeMap::new();

        let actions = compute_actions(&meta, &new_files, tmp.path()).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], UpdateAction::Remove(p) if p == Path::new("old-file.txt")));
    }

    #[test]
    fn compute_actions_unchanged_file_no_action() {
        let tmp = TempDir::new().unwrap();
        let content = b"same content";
        fs::write(tmp.path().join("file.txt"), content).unwrap();

        let meta = ProjectMeta {
            source: SourceInfo {
                template: "test".to_string(),
                remote: None,
                git_ref: None,
                version: None,
                fledge_version: "0.3.0".to_string(),
                created: "2026-04-19".to_string(),
                updated: None,
            },
            variables: BTreeMap::new(),
            files: BTreeMap::from([("file.txt".to_string(), compute_file_hash(content))]),
        };

        let mut new_files = BTreeMap::new();
        new_files.insert("file.txt".to_string(), content.to_vec());

        let actions = compute_actions(&meta, &new_files, tmp.path()).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn write_project_meta_creates_file() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("file.txt"), "hello").unwrap();

        let mut ctx = tera::Context::new();
        ctx.insert("project_name", "test-project");
        ctx.insert("author", "Test");

        write_project_meta(
            &project_dir,
            "rust-cli",
            None,
            None,
            Some("1.0.0"),
            &ctx,
            &[PathBuf::from("file.txt")],
        )
        .unwrap();

        let meta_path = project_dir.join(".fledge.toml");
        assert!(meta_path.exists());

        let content = fs::read_to_string(&meta_path).unwrap();
        let meta: ProjectMeta = toml::from_str(&content).unwrap();
        assert_eq!(meta.source.template, "rust-cli");
        assert_eq!(meta.source.version.as_deref(), Some("1.0.0"));
        assert_eq!(meta.variables["project_name"], "test-project");
        assert!(meta.files.contains_key("file.txt"));
    }

    #[test]
    fn write_project_meta_with_remote() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        fs::create_dir(&project_dir).unwrap();

        let ctx = tera::Context::new();

        write_project_meta(
            &project_dir,
            "rust-cli",
            Some("CorvidLabs/templates/rust-cli"),
            Some("v1.0"),
            None,
            &ctx,
            &[],
        )
        .unwrap();

        let content = fs::read_to_string(project_dir.join(".fledge.toml")).unwrap();
        let meta: ProjectMeta = toml::from_str(&content).unwrap();
        assert_eq!(
            meta.source.remote.as_deref(),
            Some("CorvidLabs/templates/rust-cli")
        );
        assert_eq!(meta.source.git_ref.as_deref(), Some("v1.0"));
    }
}
