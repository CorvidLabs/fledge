use anyhow::{bail, Context, Result};
use regex_lite::Regex;
use std::path::Path;
use std::process::Command;

use crate::run::detect_project_type;
use crate::versioning::{parse_version, Version};

use super::toml_utils::{extract_toml_version, extract_versioned_toml_section};

pub(crate) fn resolve_target_version(dir: &Path, bump: &str) -> Result<Version> {
    match bump {
        "major" | "minor" | "patch" => {
            let current = detect_current_version(dir)?;
            apply_bump(&current, bump)
        }
        _ => parse_version(bump),
    }
}

pub(crate) fn detect_current_version(dir: &Path) -> Result<Version> {
    // plugin.toml is the canonical fledge-ecosystem identity — prefer it over
    // any language-specific manifest. Rust plugins keep both Cargo.toml and
    // plugin.toml in sync; the plugin.toml version is the source of truth.
    if dir.join("plugin.toml").exists() {
        if let Ok(v) = read_plugin_toml_version(dir) {
            return parse_version(&v);
        }
    }

    let project_type = detect_project_type(dir);

    let version_str = match project_type {
        "rust" => read_cargo_version(dir)?,
        "node" => read_package_json_version(dir)?,
        "python" => read_python_version(dir)?,
        "ruby" => read_gemspec_version(dir)?,
        "java-gradle" => read_gradle_version(dir)?,
        "java-maven" => read_maven_version(dir)?,
        _ => read_version_from_tag(dir)?,
    };

    parse_version(&version_str)
}

pub(crate) fn read_cargo_version(dir: &Path) -> Result<String> {
    let content = std::fs::read_to_string(dir.join("Cargo.toml")).context("reading Cargo.toml")?;
    extract_toml_version(&content)
        .ok_or_else(|| anyhow::anyhow!("No version field found in Cargo.toml"))
}

/// Read `[plugin].version` from a fledge plugin manifest. Looks for the field
/// inside (or just after) the `[plugin]` table header so we don't accidentally
/// match a `version = "..."` line in a different table (e.g. a `[[commands]]`).
pub(crate) fn read_plugin_toml_version(dir: &Path) -> Result<String> {
    let content =
        std::fs::read_to_string(dir.join("plugin.toml")).context("reading plugin.toml")?;
    extract_versioned_toml_section(&content, "plugin")
        .ok_or_else(|| anyhow::anyhow!("No [plugin].version field found in plugin.toml"))
}

pub(crate) fn read_package_json_version(dir: &Path) -> Result<String> {
    let content =
        std::fs::read_to_string(dir.join("package.json")).context("reading package.json")?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    json["version"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No version field in package.json"))
}

pub(crate) fn read_python_version(dir: &Path) -> Result<String> {
    if let Ok(content) = std::fs::read_to_string(dir.join("pyproject.toml")) {
        if let Some(v) = extract_toml_version(&content) {
            return Ok(v);
        }
    }
    if let Ok(content) = std::fs::read_to_string(dir.join("setup.cfg")) {
        let re = Regex::new(r#"version\s*=\s*(\S+)"#).unwrap();
        if let Some(caps) = re.captures(&content) {
            return Ok(caps[1].to_string());
        }
    }
    read_version_from_tag(dir)
}

pub(crate) fn read_gemspec_version(dir: &Path) -> Result<String> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "gemspec") {
            let content = std::fs::read_to_string(&path)?;
            let re = Regex::new(r#"\.version\s*=\s*["']([^"']+)["']"#).unwrap();
            if let Some(caps) = re.captures(&content) {
                return Ok(caps[1].to_string());
            }
        }
    }
    read_version_from_tag(dir)
}

pub(crate) fn read_gradle_version(dir: &Path) -> Result<String> {
    for name in &["build.gradle.kts", "build.gradle"] {
        if let Ok(content) = std::fs::read_to_string(dir.join(name)) {
            let re = Regex::new(r#"version\s*=\s*["']([^"']+)["']"#).unwrap();
            if let Some(caps) = re.captures(&content) {
                return Ok(caps[1].to_string());
            }
        }
    }
    read_version_from_tag(dir)
}

pub(crate) fn read_maven_version(dir: &Path) -> Result<String> {
    let content = std::fs::read_to_string(dir.join("pom.xml")).context("reading pom.xml")?;
    let re = Regex::new(r"<version>([^<]+)</version>").unwrap();
    if let Some(caps) = re.captures(&content) {
        return Ok(caps[1].to_string());
    }
    read_version_from_tag(dir)
}

pub(crate) fn read_version_from_tag(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(dir)
        .output()
        .context("running git describe")?;

    if output.status.success() {
        let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let v = tag.strip_prefix('v').unwrap_or(&tag);
        return Ok(v.to_string());
    }

    bail!("No version found in project files or git tags. Specify an explicit version: fledge release 1.0.0")
}

pub(crate) fn apply_bump(current: &Version, bump: &str) -> Result<Version> {
    match bump {
        "major" => Ok(Version {
            major: current.major + 1,
            minor: 0,
            patch: 0,
        }),
        "minor" => Ok(Version {
            major: current.major,
            minor: current.minor + 1,
            patch: 0,
        }),
        "patch" => Ok(Version {
            major: current.major,
            minor: current.minor,
            patch: current.patch + 1,
        }),
        other => bail!(
            "Unknown bump level '{}'. Expected major, minor, or patch",
            other
        ),
    }
}
