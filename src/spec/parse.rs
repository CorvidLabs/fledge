use anyhow::{bail, Context, Result};

use super::SpecFrontmatter;

pub(crate) fn parse_frontmatter(content: &str) -> Result<(SpecFrontmatter, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        bail!("No YAML frontmatter found (must start with ---)");
    }

    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("No closing --- for frontmatter"))?;

    let yaml_str = &after_first[..end];
    let body = &after_first[end + 4..];

    let fm = parse_yaml_frontmatter(yaml_str)?;
    Ok((fm, body.to_string()))
}

pub(crate) fn parse_yaml_frontmatter(yaml: &str) -> Result<SpecFrontmatter> {
    let mut module = None;
    let mut version = None;
    let mut status = None;
    let mut files = Vec::new();
    let mut current_list: Option<&str> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- ") {
            let value = rest.trim().to_string();
            if current_list == Some("files") {
                files.push(value);
            }
            continue;
        }

        current_list = None;

        if let Some((key, val)) = trimmed.split_once(':') {
            let key = key.trim();
            let val = val.trim();

            if val.is_empty() || val == "[]" {
                if key == "files" {
                    if val == "[]" {
                        files.clear();
                    } else {
                        current_list = Some("files");
                    }
                }
                continue;
            }

            match key {
                "module" => module = Some(val.to_string()),
                "version" => {
                    version = Some(
                        val.parse::<u32>()
                            .with_context(|| format!("Invalid version: {val}"))?,
                    );
                }
                "status" => status = Some(val.to_string()),
                "files" if val.starts_with('[') && val.ends_with(']') => {
                    let inner = &val[1..val.len() - 1];
                    files = inner
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    Ok(SpecFrontmatter {
        module: module.ok_or_else(|| anyhow::anyhow!("Missing required field: module"))?,
        version: version.ok_or_else(|| anyhow::anyhow!("Missing required field: version"))?,
        status: status.ok_or_else(|| anyhow::anyhow!("Missing required field: status"))?,
        files,
    })
}

pub(crate) fn extract_sections(body: &str) -> Vec<String> {
    let mut sections = Vec::new();
    for line in body.lines() {
        if let Some(section) = line.strip_prefix("## ") {
            sections.push(section.trim().to_string());
        }
    }
    sections
}

pub(crate) fn extract_purpose(body: &str) -> Option<String> {
    let mut in_purpose = false;
    let mut paragraph = String::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## ") {
            if in_purpose {
                break;
            }
            if trimmed == "## Purpose" {
                in_purpose = true;
            }
            continue;
        }
        if !in_purpose {
            continue;
        }
        if line.trim().is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(line.trim());
    }
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph)
    }
}
