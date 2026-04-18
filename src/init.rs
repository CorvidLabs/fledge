use anyhow::{Context, Result, bail};
use console::style;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::prompts;
use crate::templates::{self, Template};

pub struct InitOptions {
    pub name: String,
    pub template: Option<String>,
    pub output: PathBuf,
    pub no_git: bool,
    #[allow(dead_code)]
    pub no_install: bool,
}

pub fn run(opts: InitOptions) -> Result<()> {
    let config = Config::load().context("loading config")?;
    let extra_paths = config.extra_template_paths();
    let available = templates::discover_templates(&extra_paths)?;

    if available.is_empty() {
        bail!("No templates found. Add templates to the templates/ directory.");
    }

    // Resolve which template to use
    let template = resolve_template(&available, opts.template.as_deref())?;

    println!(
        "{} Using template: {}",
        style("*").cyan().bold(),
        style(&template.name).green()
    );

    // Target directory
    let target_dir = opts.output.join(&opts.name);
    if target_dir.exists() {
        bail!(
            "Directory '{}' already exists. Choose a different name or remove it first.",
            target_dir.display()
        );
    }

    // Prompt for template variables
    let variables = prompts::prompt_variables(template, &opts.name, &config)?;

    // Create project directory
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating directory {}", target_dir.display()))?;

    // Render template
    println!("{} Scaffolding project...", style("*").cyan().bold());
    let created_files = templates::render_template(template, &target_dir, &variables)?;

    // Git init
    if !opts.no_git {
        init_git(&target_dir)?;
    }

    // Print summary
    println!();
    println!(
        "{} Created {} in {}",
        style("✓").green().bold(),
        style(&opts.name).cyan().bold(),
        style(target_dir.display()).dim()
    );
    println!();
    for file in &created_files {
        println!("  {}", style(file.display()).dim());
    }
    println!();
    println!("  {} files created", created_files.len());

    if !opts.no_git {
        println!("  git repo initialized with initial commit");
    }

    println!();
    println!("  cd {} && get started!", style(&opts.name).cyan());

    Ok(())
}

fn resolve_template<'a>(
    available: &'a [Template],
    requested: Option<&str>,
) -> Result<&'a Template> {
    match requested {
        Some(name) => available.iter().find(|t| t.name == name).ok_or_else(|| {
            let names: Vec<&str> = available.iter().map(|t| t.name.as_str()).collect();
            anyhow::anyhow!(
                "Template '{}' not found. Available: {}",
                name,
                names.join(", ")
            )
        }),
        None => {
            let idx = prompts::select_template(available)?;
            Ok(&available[idx])
        }
    }
}

fn init_git(dir: &Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running git init")?;

    if !status.success() {
        bail!("git init failed");
    }

    // Stage all files
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running git add")?;

    // Initial commit
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit from fledge"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running git commit")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_test_templates(dir: &Path) -> PathBuf {
        let tpl_dir = dir.join("templates");
        fs::create_dir_all(&tpl_dir).unwrap();

        let test_tpl = tpl_dir.join("test-tpl");
        fs::create_dir(&test_tpl).unwrap();
        fs::write(
            test_tpl.join("template.toml"),
            r#"
[template]
name = "test-tpl"
description = "Test template"

[files]
render = ["**/*.md"]
ignore = ["template.toml"]
"#,
        )
        .unwrap();
        fs::write(test_tpl.join("README.md"), "# {{ project_name }}").unwrap();
        fs::write(test_tpl.join("plain.txt"), "no rendering").unwrap();

        tpl_dir
    }

    #[test]
    fn resolve_template_by_name() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = make_test_templates(tmp.path());
        let templates = templates::discover_templates(&[tpl_dir]).unwrap();

        let result = resolve_template(&templates, Some("test-tpl"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test-tpl");
    }

    #[test]
    fn resolve_template_unknown_name_errors() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = make_test_templates(tmp.path());
        let templates = templates::discover_templates(&[tpl_dir]).unwrap();

        let result = resolve_template(&templates, Some("nonexistent"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"));
        assert!(err.contains("not found"));
    }

    #[test]
    fn resolve_template_error_lists_available() {
        let tmp = TempDir::new().unwrap();
        let tpl_dir = make_test_templates(tmp.path());
        let templates = templates::discover_templates(&[tpl_dir]).unwrap();

        let err = resolve_template(&templates, Some("missing"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("test-tpl"));
    }

    #[test]
    fn init_git_creates_repo() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-project");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file.txt"), "hello").unwrap();

        let result = init_git(&dir);
        assert!(result.is_ok());
        assert!(dir.join(".git").exists());
    }

    #[test]
    fn init_git_makes_initial_commit() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-project");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file.txt"), "hello").unwrap();

        init_git(&dir).unwrap();

        let output = std::process::Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let log = String::from_utf8(output.stdout).unwrap();
        assert!(log.contains("Initial commit from fledge"));
    }
}
