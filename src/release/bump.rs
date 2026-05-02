use anyhow::{bail, Context, Result};
use regex_lite::Regex;
use std::path::Path;
use std::process::Command;

use crate::versioning::Version;

use super::toml_utils::replace_versioned_toml_section;
use super::version::detect_current_version;
use super::BumpResult;

pub(super) fn detect_version_files(dir: &Path) -> Vec<String> {
    let candidates: &[(&str, &str)] = &[
        ("plugin.toml", "fledge-plugin"),
        ("Cargo.toml", "rust"),
        ("package.json", "node"),
        ("pyproject.toml", "python"),
        ("setup.cfg", "python"),
        ("pom.xml", "java-maven"),
        ("build.gradle", "java-gradle"),
        ("build.gradle.kts", "java-gradle"),
    ];

    let mut files: Vec<String> = candidates
        .iter()
        .filter(|(name, _)| dir.join(name).exists())
        .map(|(name, _)| name.to_string())
        .collect();

    // Mirror `bump_version_files`'s `[release].files` handling so the dry-run
    // envelope reports the same set the real release would write. Without this,
    // `release --dry-run --json` says `files_to_bump: ["Cargo.toml"]` while a
    // subsequent real run also bumps `flake.nix` (or whatever else is listed),
    // breaking the contract that dry-run accurately previews the release.
    if let Ok(content) = std::fs::read_to_string(dir.join("fledge.toml")) {
        if let Ok(parsed) = content.parse::<toml::Value>() {
            if let Some(extras) = parsed
                .get("release")
                .and_then(|r| r.get("files"))
                .and_then(|f| f.as_array())
            {
                // Same regex `bump_version_files` uses for extras — reporting
                // only files that actually have a parseable version line keeps
                // dry-run honest.
                let re = Regex::new(r#"(?m)(version\s*[=:]\s*["']?)(\d+\.\d+\.\d+)"#).unwrap();
                for entry in extras {
                    let Some(name) = entry.as_str() else { continue };
                    if files.iter().any(|f| f == name) {
                        continue;
                    }
                    let path = dir.join(name);
                    if !path.exists() {
                        continue;
                    }
                    if let Ok(file_content) = std::fs::read_to_string(&path) {
                        if re.is_match(&file_content) {
                            files.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    files
}

pub(super) fn bump_version_files(dir: &Path, new_version: &Version) -> Result<BumpResult> {
    let old = detect_current_version(dir).unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
    });
    let new_str = new_version.to_string();
    let mut bumped = Vec::new();

    // plugin.toml: only touch the version field inside the [plugin] table.
    // Other tables (e.g. `[[commands]]`) may have their own `version` and we
    // must not rewrite those.
    if let Ok(content) = std::fs::read_to_string(dir.join("plugin.toml")) {
        if let Some(updated) = replace_versioned_toml_section(&content, "plugin", &new_str) {
            std::fs::write(dir.join("plugin.toml"), updated.as_bytes())?;
            bumped.push("plugin.toml".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("Cargo.toml")) {
        let re = Regex::new(r#"(?m)^(version\s*=\s*")[^"]+(")"#).unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}${{2}}"));
            std::fs::write(dir.join("Cargo.toml"), updated.as_bytes())?;
            bumped.push("Cargo.toml".to_string());

            if dir.join("Cargo.lock").exists() {
                let lock = std::fs::read_to_string(dir.join("Cargo.lock"))?;
                if lock.contains(&format!("version = \"{}\"", old)) {
                    let status = Command::new("cargo")
                        .args(["generate-lockfile"])
                        .current_dir(dir)
                        .status()
                        .with_context(|| "running cargo generate-lockfile")?;
                    if status.success() {
                        bumped.push("Cargo.lock".to_string());
                    } else {
                        eprintln!("Warning: cargo generate-lockfile failed, Cargo.lock not staged");
                    }
                }
            }
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
        let re = Regex::new(r#"("version"\s*:\s*")[^"]+(")"#).unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}${{2}}"));
            std::fs::write(dir.join("package.json"), updated.as_bytes())?;
            bumped.push("package.json".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("pyproject.toml")) {
        let re = Regex::new(r#"(?m)^(version\s*=\s*")[^"]+(")"#).unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}${{2}}"));
            std::fs::write(dir.join("pyproject.toml"), updated.as_bytes())?;
            bumped.push("pyproject.toml".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("setup.cfg")) {
        let re = Regex::new(r"(?m)^(version\s*=\s*)\S+").unwrap();
        if re.is_match(&content) {
            let updated = re.replace(&content, format!("${{1}}{new_str}"));
            std::fs::write(dir.join("setup.cfg"), updated.as_bytes())?;
            bumped.push("setup.cfg".to_string());
        }
    }

    if let Ok(content) = std::fs::read_to_string(dir.join("pom.xml")) {
        let re = Regex::new(r"(<version>)([^<]+)(</version>)").unwrap();
        let old_version_str = old.to_string();
        if let Some(caps) = re.captures(&content) {
            if caps
                .get(2)
                .map(|m| m.as_str().trim() == old_version_str.as_str())
                .unwrap_or(false)
            {
                let updated = re.replace(&content, format!("${{1}}{new_str}${{3}}"));
                std::fs::write(dir.join("pom.xml"), updated.as_bytes())?;
                bumped.push("pom.xml".to_string());
            }
        }
    }

    for name in &["build.gradle", "build.gradle.kts"] {
        let path = dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let re = Regex::new(r#"(version\s*=\s*)(["'])[^"']+["']"#).unwrap();
            if re.is_match(&content) {
                let updated = re.replace(&content, format!("${{1}}${{2}}{new_str}${{2}}"));
                std::fs::write(&path, updated.as_bytes())?;
                bumped.push(name.to_string());
            }
        }
    }

    // Also check for [release] config in fledge.toml for custom version files
    if let Ok(content) = std::fs::read_to_string(dir.join("fledge.toml")) {
        if let Ok(parsed) = content.parse::<toml::Value>() {
            if let Some(files) = parsed
                .get("release")
                .and_then(|r| r.get("files"))
                .and_then(|f| f.as_array())
            {
                for file_val in files {
                    if let Some(file_name) = file_val.as_str() {
                        if !bumped.iter().any(|b| b == file_name) {
                            let path = dir.join(file_name);
                            if path.exists() {
                                let canonical_path = path
                                    .canonicalize()
                                    .with_context(|| format!("canonicalizing '{}'", file_name))?;
                                let canonical_dir = dir
                                    .canonicalize()
                                    .with_context(|| "canonicalizing project dir")?;
                                if !canonical_path.starts_with(&canonical_dir) {
                                    bail!("Release file '{}' escapes project directory", file_name);
                                }
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let re = Regex::new(
                                        r#"(?m)(version\s*[=:]\s*["']?)(\d+\.\d+\.\d+)"#,
                                    )
                                    .unwrap();
                                    if re.is_match(&content) {
                                        let updated =
                                            re.replace(&content, format!("${{1}}{new_str}"));
                                        std::fs::write(&path, updated.as_bytes())?;
                                        bumped.push(file_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(BumpResult {
        old,
        new: new_version.clone(),
        files_bumped: bumped,
    })
}
