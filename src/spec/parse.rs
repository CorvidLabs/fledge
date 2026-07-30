use anyhow::{bail, Context, Result};

use super::SpecFrontmatter;

pub(crate) fn parse_frontmatter(content: &str) -> Result<(SpecFrontmatter, String)> {
    let (yaml_str, body) = split_frontmatter(content)?;
    let fm = parse_yaml_frontmatter(&yaml_str)?;
    Ok((fm, body))
}

/// Split a spec file into its raw YAML frontmatter block and its markdown body,
/// without interpreting either. `parse_frontmatter` layers typed parsing on top;
/// `lint` needs the raw block so it can inspect fields (notably `version`) that
/// the typed parser would reject outright.
pub(crate) fn split_frontmatter(content: &str) -> Result<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        bail!("No YAML frontmatter found (must start with ---)");
    }

    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("No closing --- for frontmatter"))?;

    Ok((
        after_first[..end].to_string(),
        after_first[end + 4..].to_string(),
    ))
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

/// Extract every `## Section` heading paired with its body — everything up to
/// the next `## ` heading (so `###` subsections are part of their parent's
/// body). Order is preserved; a repeated heading yields repeated entries.
///
/// `extract_sections` answers "which sections exist"; this answers "and what is
/// in them", which is what the emptiness checks in `lint` need.
pub(crate) fn extract_section_bodies(body: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some((name, lines)) = current.take() {
                sections.push((name, lines.join("\n")));
            }
            current = Some((heading.trim().to_string(), Vec::new()));
            continue;
        }
        if let Some((_, lines)) = current.as_mut() {
            lines.push(line);
        }
    }
    if let Some((name, lines)) = current.take() {
        sections.push((name, lines.join("\n")));
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
