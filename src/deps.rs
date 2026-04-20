use anyhow::{Context, Result, bail};
use console::style;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::run::detect_project_type;

pub struct DepsOptions {
    pub outdated: bool,
    pub audit: bool,
    pub licenses: bool,
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct DepsReport {
    ecosystem: String,
    source: Option<String>,
    dependencies: Vec<Dep>,
}

#[derive(Debug, Serialize)]
struct Dep {
    name: String,
    version: String,
}

pub fn run(opts: DepsOptions) -> Result<()> {
    let project_dir = std::env::current_dir().context("getting current directory")?;
    let project_type = detect_project_type(&project_dir);

    if project_type == "generic" {
        bail!(
            "Could not detect project type. Supported: Rust, Node, Go, Python, Ruby, Java (Gradle/Maven)."
        );
    }

    if opts.outdated {
        return run_outdated(project_type, &project_dir);
    }

    if opts.audit {
        return run_audit(project_type, &project_dir);
    }

    let (lock_file, deps) = parse_dependencies(project_type, &project_dir)?;

    if opts.licenses {
        return run_licenses(project_type, &project_dir);
    }

    if opts.json {
        let report = DepsReport {
            ecosystem: project_type.to_string(),
            source: lock_file.map(|p| p.to_string_lossy().to_string()),
            dependencies: deps,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let source_label = lock_file
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|| "manifest".to_string());

    println!(
        "\n{} ({} via {})",
        style("Dependencies").bold(),
        style(project_type).cyan(),
        style(&source_label).dim()
    );
    println!("  {} {}\n", style("Total:").bold(), deps.len());

    if deps.is_empty() {
        println!("  {}", style("No dependencies found.").dim());
        return Ok(());
    }

    let max_name = deps.iter().map(|d| d.name.len()).max().unwrap_or(20).max(4);
    println!(
        "  {:<width$}  {}",
        style("Name").underlined(),
        style("Version").underlined(),
        width = max_name
    );
    for dep in &deps {
        println!("  {:<width$}  {}", dep.name, dep.version, width = max_name);
    }
    println!();

    Ok(())
}

fn parse_dependencies(
    project_type: &str,
    project_dir: &Path,
) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    match project_type {
        "rust" => parse_cargo_lock(project_dir),
        "node" => parse_node_lock(project_dir),
        "go" => parse_go_sum(project_dir),
        "python" => parse_python_deps(project_dir),
        "ruby" => parse_gemfile_lock(project_dir),
        "java-gradle" | "java-maven" => {
            println!(
                "  {} Lock file parsing not supported for {}. Use {} or {} instead.",
                style("*").cyan().bold(),
                project_type,
                style("--outdated").cyan(),
                style("--audit").cyan()
            );
            Ok((None, vec![]))
        }
        _ => bail!("Unsupported project type: {}", project_type),
    }
}

fn parse_cargo_lock(dir: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let lock_path = dir.join("Cargo.lock");
    if !lock_path.exists() {
        bail!("No Cargo.lock found. Run `cargo generate-lockfile` first.");
    }

    let content = std::fs::read_to_string(&lock_path).context("reading Cargo.lock")?;
    let mut deps = Vec::new();
    let mut current_name: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("name = ") {
            current_name = Some(unquote(line.trim_start_matches("name = ")));
        } else if line.starts_with("version = ") {
            if let Some(name) = current_name.take() {
                let version = unquote(line.trim_start_matches("version = "));
                deps.push(Dep { name, version });
            }
        } else if line == "[[package]]" {
            current_name = None;
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(lock_path), deps))
}

fn parse_node_lock(dir: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let npm_lock = dir.join("package-lock.json");
    if npm_lock.exists() {
        return parse_npm_lock(&npm_lock);
    }

    let yarn_lock = dir.join("yarn.lock");
    if yarn_lock.exists() {
        return parse_yarn_lock(&yarn_lock);
    }

    let pnpm_lock = dir.join("pnpm-lock.yaml");
    if pnpm_lock.exists() {
        bail!(
            "pnpm-lock.yaml detected but YAML parsing is not supported. Use `pnpm outdated` or `pnpm audit` directly."
        );
    }

    bail!(
        "No lock file found (package-lock.json, yarn.lock, or pnpm-lock.yaml). Run your package manager's install command first."
    );
}

fn parse_npm_lock(path: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let content = std::fs::read_to_string(path).context("reading package-lock.json")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).context("parsing package-lock.json")?;

    let mut deps = Vec::new();

    if let Some(packages) = parsed.get("packages").and_then(|p| p.as_object()) {
        for (key, val) in packages {
            if key.is_empty() {
                continue;
            }
            let name = key.strip_prefix("node_modules/").unwrap_or(key).to_string();
            let version = val
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            deps.push(Dep { name, version });
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(path.to_path_buf()), deps))
}

fn parse_yarn_lock(path: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let content = std::fs::read_to_string(path).context("reading yarn.lock")?;
    let mut deps = Vec::new();
    let mut current_name: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && !trimmed.is_empty() && !line.starts_with(' ') {
            let raw = trimmed.trim_end_matches(':');
            let name = raw
                .split('@')
                .next()
                .unwrap_or(raw)
                .trim_matches('"')
                .to_string();
            if !name.is_empty() {
                current_name = Some(name);
            }
        } else if trimmed.starts_with("version ") {
            if let Some(name) = current_name.take() {
                let version = unquote(trimmed.trim_start_matches("version "));
                if !deps.iter().any(|d: &Dep| d.name == name) {
                    deps.push(Dep { name, version });
                }
            }
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(path.to_path_buf()), deps))
}

fn parse_go_sum(dir: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let sum_path = dir.join("go.sum");
    if !sum_path.exists() {
        bail!("No go.sum found. Run `go mod tidy` first.");
    }

    let content = std::fs::read_to_string(&sum_path).context("reading go.sum")?;
    let mut deps = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let module = parts[0].to_string();
            let version = parts[1]
                .strip_suffix("/go.mod")
                .unwrap_or(parts[1])
                .to_string();
            let key = format!("{module}@{version}");
            if seen.insert(key) {
                deps.push(Dep {
                    name: module,
                    version,
                });
            }
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(sum_path), deps))
}

fn parse_python_deps(dir: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let req_path = dir.join("requirements.txt");
    if req_path.exists() {
        return parse_requirements_txt(&req_path);
    }

    let pipfile_lock = dir.join("Pipfile.lock");
    if pipfile_lock.exists() {
        return parse_pipfile_lock(&pipfile_lock);
    }

    let poetry_lock = dir.join("poetry.lock");
    if poetry_lock.exists() {
        return parse_poetry_lock(&poetry_lock);
    }

    bail!(
        "No Python lock/requirements file found (requirements.txt, Pipfile.lock, or poetry.lock)."
    );
}

fn parse_requirements_txt(path: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let content = std::fs::read_to_string(path).context("reading requirements.txt")?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }
        if let Some((name, version)) = trimmed.split_once("==") {
            deps.push(Dep {
                name: name.trim().to_string(),
                version: version.trim().to_string(),
            });
        } else if let Some((name, version)) = trimmed.split_once(">=") {
            deps.push(Dep {
                name: name.trim().to_string(),
                version: format!(">={}", version.trim()),
            });
        } else {
            deps.push(Dep {
                name: trimmed.to_string(),
                version: "*".to_string(),
            });
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(path.to_path_buf()), deps))
}

fn parse_pipfile_lock(path: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let content = std::fs::read_to_string(path).context("reading Pipfile.lock")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).context("parsing Pipfile.lock")?;
    let mut deps = Vec::new();

    if let Some(default) = parsed.get("default").and_then(|d| d.as_object()) {
        for (name, val) in default {
            let version = val
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .trim_start_matches("==")
                .to_string();
            deps.push(Dep {
                name: name.clone(),
                version,
            });
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(path.to_path_buf()), deps))
}

fn parse_poetry_lock(path: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let content = std::fs::read_to_string(path).context("reading poetry.lock")?;
    let mut deps = Vec::new();
    let mut current_name: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("name = ") {
            current_name = Some(unquote(line.trim_start_matches("name = ")));
        } else if line.starts_with("version = ") {
            if let Some(name) = current_name.take() {
                let version = unquote(line.trim_start_matches("version = "));
                deps.push(Dep { name, version });
            }
        } else if line == "[[package]]" {
            current_name = None;
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(path.to_path_buf()), deps))
}

fn parse_gemfile_lock(dir: &Path) -> Result<(Option<PathBuf>, Vec<Dep>)> {
    let lock_path = dir.join("Gemfile.lock");
    if !lock_path.exists() {
        bail!("No Gemfile.lock found. Run `bundle install` first.");
    }

    let content = std::fs::read_to_string(&lock_path).context("reading Gemfile.lock")?;
    let mut deps = Vec::new();
    let mut in_specs = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "specs:" {
            in_specs = true;
            continue;
        }
        if in_specs {
            if !line.starts_with(' ') && !line.starts_with('\t') {
                in_specs = false;
                continue;
            }
            // Gem entries are indented 4 spaces: "    gem_name (version)"
            if line.starts_with("    ") && !line.starts_with("      ") {
                if let Some((name, rest)) = trimmed.split_once(' ') {
                    let version = rest.trim_matches(|c| c == '(' || c == ')').to_string();
                    deps.push(Dep {
                        name: name.to_string(),
                        version,
                    });
                }
            }
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Some(lock_path), deps))
}

fn run_outdated(project_type: &str, dir: &Path) -> Result<()> {
    let (cmd, args, tool_name) = match project_type {
        "rust" => ("cargo", vec!["outdated"], "cargo-outdated"),
        "node" => ("npm", vec!["outdated"], "npm"),
        "go" => ("go", vec!["list", "-m", "-u", "all"], "go"),
        "python" => ("pip", vec!["list", "--outdated"], "pip"),
        "ruby" => ("bundle", vec!["outdated"], "bundler"),
        _ => bail!("Outdated check not supported for {}", project_type),
    };

    run_ecosystem_command(cmd, &args, dir, tool_name, "outdated check")
}

fn run_audit(project_type: &str, dir: &Path) -> Result<()> {
    let (cmd, args, tool_name) = match project_type {
        "rust" => ("cargo", vec!["audit"], "cargo-audit"),
        "node" => ("npm", vec!["audit"], "npm"),
        "go" => ("govulncheck", vec!["./..."], "govulncheck"),
        "python" => ("pip-audit", vec![], "pip-audit"),
        "ruby" => ("bundle", vec!["audit", "check"], "bundler-audit"),
        _ => bail!("Security audit not supported for {}", project_type),
    };

    run_ecosystem_command(cmd, &args, dir, tool_name, "security audit")
}

fn run_licenses(project_type: &str, dir: &Path) -> Result<()> {
    let (cmd, args, tool_name) = match project_type {
        "rust" => ("cargo", vec!["license"], "cargo-license"),
        "node" => (
            "npx",
            vec!["license-checker", "--summary"],
            "license-checker",
        ),
        "go" => ("go-licenses", vec!["report", "./..."], "go-licenses"),
        "python" => ("pip-licenses", vec![], "pip-licenses"),
        "ruby" => ("bundle", vec!["exec", "license_finder"], "license_finder"),
        _ => bail!("License scanning not supported for {}", project_type),
    };

    run_ecosystem_command(cmd, &args, dir, tool_name, "license scan")
}

fn run_ecosystem_command(
    cmd: &str,
    args: &[&str],
    dir: &Path,
    tool_name: &str,
    action: &str,
) -> Result<()> {
    println!(
        "{} Running {} ({} {})...\n",
        style("▸").cyan().bold(),
        style(action).bold(),
        cmd,
        args.join(" ")
    );

    let result = Command::new(cmd).args(args).current_dir(dir).status();

    match result {
        Ok(status) => {
            if !status.success() {
                let code = status.code().unwrap_or(1);
                if code != 0 {
                    println!(
                        "\n{} {} exited with code {} (this may indicate issues were found)",
                        style("!").yellow().bold(),
                        tool_name,
                        code
                    );
                }
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!(
                "'{}' not found. Install it to use {}.\n  See: {} docs for installation instructions.",
                cmd,
                action,
                tool_name,
            );
        }
        Err(e) => Err(e).with_context(|| format!("running {cmd}")),
    }
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cargo_lock_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        std::fs::write(
            dir.path().join("Cargo.lock"),
            r#"# This file is generated
[[package]]
name = "anyhow"
version = "1.0.86"

[[package]]
name = "clap"
version = "4.5.0"
"#,
        )
        .unwrap();

        let (lock, deps) = parse_cargo_lock(dir.path()).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "anyhow");
        assert_eq!(deps[0].version, "1.0.86");
        assert_eq!(deps[1].name, "clap");
        assert_eq!(deps[1].version, "4.5.0");
    }

    #[test]
    fn parse_cargo_lock_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = parse_cargo_lock(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn parse_npm_lock_basic() {
        let dir = tempfile::tempdir().unwrap();
        let lock_content = serde_json::json!({
            "name": "test",
            "lockfileVersion": 3,
            "packages": {
                "": { "name": "test", "version": "1.0.0" },
                "node_modules/express": { "version": "4.18.0" },
                "node_modules/lodash": { "version": "4.17.21" }
            }
        });
        let lock_path = dir.path().join("package-lock.json");
        std::fs::write(
            &lock_path,
            serde_json::to_string_pretty(&lock_content).unwrap(),
        )
        .unwrap();

        let (lock, deps) = parse_npm_lock(&lock_path).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "express");
        assert_eq!(deps[1].name, "lodash");
    }

    #[test]
    fn parse_go_sum_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module test").unwrap();
        std::fs::write(
            dir.path().join("go.sum"),
            "github.com/pkg/errors v0.9.1 h1:abc=\ngithub.com/pkg/errors v0.9.1/go.mod h1:def=\n",
        )
        .unwrap();

        let (lock, deps) = parse_go_sum(dir.path()).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "github.com/pkg/errors");
        assert_eq!(deps[0].version, "v0.9.1");
    }

    #[test]
    fn parse_requirements_txt_basic() {
        let dir = tempfile::tempdir().unwrap();
        let req_path = dir.path().join("requirements.txt");
        std::fs::write(
            &req_path,
            "# comment\nflask==2.3.0\nrequests>=2.28.0\nnumpy\n",
        )
        .unwrap();

        let (lock, deps) = parse_requirements_txt(&req_path).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].name, "flask");
        assert_eq!(deps[0].version, "2.3.0");
        assert_eq!(deps[1].name, "numpy");
        assert_eq!(deps[1].version, "*");
        assert_eq!(deps[2].name, "requests");
        assert_eq!(deps[2].version, ">=2.28.0");
    }

    #[test]
    fn parse_pipfile_lock_basic() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("Pipfile.lock");
        let content = serde_json::json!({
            "_meta": {},
            "default": {
                "flask": { "version": "==2.3.0" },
                "requests": { "version": "==2.28.0" }
            },
            "develop": {}
        });
        std::fs::write(&lock_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

        let (lock, deps) = parse_pipfile_lock(&lock_path).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "flask");
        assert_eq!(deps[0].version, "2.3.0");
    }

    #[test]
    fn parse_gemfile_lock_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();
        std::fs::write(
            dir.path().join("Gemfile.lock"),
            r#"GEM
  remote: https://rubygems.org/
  specs:
    rails (7.0.0)
      actionpack (= 7.0.0)
    actionpack (7.0.0)

PLATFORMS
  ruby
"#,
        )
        .unwrap();

        let (lock, deps) = parse_gemfile_lock(dir.path()).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "actionpack");
        assert_eq!(deps[0].version, "7.0.0");
        assert_eq!(deps[1].name, "rails");
        assert_eq!(deps[1].version, "7.0.0");
    }

    #[test]
    fn parse_poetry_lock_basic() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("poetry.lock");
        std::fs::write(
            &lock_path,
            r#"[[package]]
name = "certifi"
version = "2024.2.2"

[[package]]
name = "urllib3"
version = "2.2.1"
"#,
        )
        .unwrap();

        let (lock, deps) = parse_poetry_lock(&lock_path).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "certifi");
        assert_eq!(deps[1].name, "urllib3");
    }

    #[test]
    fn parse_yarn_lock_basic() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("yarn.lock");
        std::fs::write(
            &lock_path,
            r#"# yarn lockfile v1

express@^4.18.0:
  version "4.18.2"
  resolved "https://registry.yarnpkg.com/express/-/express-4.18.2.tgz"

lodash@^4.17.0:
  version "4.17.21"
  resolved "https://registry.yarnpkg.com/lodash/-/lodash-4.17.21.tgz"
"#,
        )
        .unwrap();

        let (lock, deps) = parse_yarn_lock(&lock_path).unwrap();
        assert!(lock.is_some());
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "express");
        assert_eq!(deps[1].name, "lodash");
    }

    #[test]
    fn unquote_strips_quotes() {
        assert_eq!(unquote("\"hello\""), "hello");
        assert_eq!(unquote("  \"world\"  "), "world");
        assert_eq!(unquote("bare"), "bare");
    }

    #[test]
    fn generic_project_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(dir.path()), "generic");
    }
}
