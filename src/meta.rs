use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

#[allow(dead_code)]
pub fn resolve_meta_path(project_dir: &Path) -> Option<PathBuf> {
    let new_path = project_dir.join(".fledge").join("meta.toml");
    if new_path.exists() {
        return Some(new_path);
    }
    let legacy_path = project_dir.join(".fledge.toml");
    if legacy_path.exists() {
        return Some(legacy_path);
    }
    None
}

fn ensure_dot_fledge_dir(project_dir: &Path) -> Result<PathBuf> {
    let dir = project_dir.join(".fledge");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).context("creating .fledge directory")?;
    }
    Ok(dir)
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

    let toml_str = toml::to_string_pretty(&meta).context("serializing project metadata")?;
    let dot_fledge = ensure_dot_fledge_dir(project_dir)?;
    let meta_path = dot_fledge.join("meta.toml");
    std::fs::write(&meta_path, &toml_str).context("writing .fledge/meta.toml")?;

    write_dot_fledge_gitignore(&dot_fledge)?;

    Ok(())
}

fn write_dot_fledge_gitignore(dot_fledge_dir: &Path) -> Result<()> {
    let gitignore_path = dot_fledge_dir.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, "# Cache and local overrides\n/cache/\n")
            .context("writing .fledge/.gitignore")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn compute_file_hash_known_values() {
        // Test empty input
        assert_eq!(
            compute_file_hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // Test known string
        assert_eq!(
            compute_file_hash(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn compute_file_hash_is_deterministic() {
        let h1 = compute_file_hash(b"hello world");
        let h2 = compute_file_hash(b"hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn compute_file_hash_changes_with_content() {
        assert_ne!(compute_file_hash(b"a"), compute_file_hash(b"b"));
    }

    #[test]
    fn resolve_meta_path_finds_new_layout() {
        let tmp = TempDir::new().unwrap();
        let dot_fledge = tmp.path().join(".fledge");
        std::fs::create_dir_all(&dot_fledge).unwrap();
        std::fs::write(dot_fledge.join("meta.toml"), "").unwrap();
        assert!(resolve_meta_path(tmp.path()).is_some());
    }

    #[test]
    fn resolve_meta_path_finds_legacy_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".fledge.toml"), "").unwrap();
        assert!(resolve_meta_path(tmp.path()).is_some());
    }

    #[test]
    fn resolve_meta_path_missing_returns_none() {
        let tmp = TempDir::new().unwrap();
        assert!(resolve_meta_path(tmp.path()).is_none());
    }
}
