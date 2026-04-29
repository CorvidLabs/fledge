use regex_lite::Regex;

pub(crate) fn extract_toml_version(content: &str) -> Option<String> {
    let re = Regex::new(r#"(?m)^version\s*=\s*"([^"]+)""#).unwrap();
    re.captures(content).map(|c| c[1].to_string())
}

/// Extract `version = "X.Y.Z"` from a specific `[section]` table within a TOML
/// file. Stops scanning at the next table header so a later table's `version`
/// (e.g. on a `[[commands]]` entry) doesn't get picked up by accident.
pub(crate) fn extract_versioned_toml_section(content: &str, section: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut in_section = false;
    let version_re = Regex::new(r#"^\s*version\s*=\s*"([^"]+)"\s*$"#).unwrap();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
            continue;
        }
        if in_section {
            if let Some(caps) = version_re.captures(line) {
                return Some(caps[1].to_string());
            }
        }
    }
    None
}

/// Replace the `version = "..."` line scoped to a specific `[section]` table.
/// Returns `Some(new_content)` if a replacement was made, `None` if either the
/// section or its `version` line was absent (so the caller knows whether to
/// touch the file). Preserves the original line-ending (LF / CRLF).
pub(crate) fn replace_versioned_toml_section(
    content: &str,
    section: &str,
    new_version: &str,
) -> Option<String> {
    let header = format!("[{section}]");
    let version_re = Regex::new(r#"^(\s*version\s*=\s*")[^"]+("\s*)$"#).unwrap();
    let crlf = content.contains("\r\n");
    let line_sep = if crlf { "\r\n" } else { "\n" };
    let trailing_newline = content.ends_with('\n');

    let mut in_section = false;
    let mut replaced = false;
    let mut out_lines: Vec<String> = Vec::new();

    for raw in content.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
            out_lines.push(line.to_string());
            continue;
        }
        if in_section && !replaced {
            if let Some(caps) = version_re.captures(line) {
                out_lines.push(format!("{}{}{}", &caps[1], new_version, &caps[2]));
                replaced = true;
                continue;
            }
        }
        out_lines.push(line.to_string());
    }

    if !replaced {
        return None;
    }

    let mut joined = out_lines.join(line_sep);
    if trailing_newline {
        joined.push_str(line_sep);
    }
    Some(joined)
}
