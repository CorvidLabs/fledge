use anyhow::{bail, Context, Result};
use console::style;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::prompts;
use crate::templates::{self, Template};
use crate::update;

pub struct InitOptions {
    pub name: String,
    pub template: Option<String>,
    pub output: PathBuf,
    pub author: Option<String>,
    pub org: Option<String>,
    pub no_git: bool,
    pub no_install: bool,
    pub refresh: bool,
    pub dry_run: bool,
    pub yes: bool,
}

pub fn run(opts: InitOptions) -> Result<()> {
    crate::plugin::run_lifecycle_hook("pre_init").ok();
    let config = Config::load().context("loading config")?;
    let extra_paths = config.extra_template_paths();
    let token = config.github_token();
    let token_ref = token.as_deref();

    // If template looks like a remote ref, fetch it directly
    if opts
        .template
        .as_deref()
        .is_some_and(crate::remote::is_remote_ref)
    {
        let tpl_name = opts.template.as_ref().unwrap().clone();
        return run_remote(opts, &tpl_name, &config, token_ref);
    }

    let available =
        templates::discover_templates_with_repos(&extra_paths, config.template_repos(), token_ref)?;

    if available.is_empty() {
        bail!("No templates found. Add templates to the templates/ directory.");
    }

    if opts.template.is_none() && config.template_repos().is_empty() && available.len() <= 2 {
        println!(
            "{} Only built-in starter templates found. Run {} to discover more, or add {} to your config.",
            style("tip:").yellow().bold(),
            style("fledge search").cyan(),
            style("CorvidLabs/fledge-templates").cyan(),
        );
    }

    // Resolve which template to use
    let template = resolve_template(&available, opts.template.as_deref())?;

    println!(
        "{} Using template: {}",
        style("*").cyan().bold(),
        style(&template.name).green()
    );

    check_template_version(&template.manifest)?;
    let reqs_ok = check_template_requirements(&template.manifest, opts.yes)?;

    // Target directory
    let target_dir = opts.output.join(&opts.name);
    if target_dir.exists() {
        bail!(
            "Directory '{}' already exists. Choose a different name or remove it first.",
            target_dir.display()
        );
    }

    if opts.dry_run {
        return print_dry_run(
            template,
            &target_dir,
            &template.manifest.hooks.post_create,
            opts.no_git,
            false,
        );
    }

    // Prompt for template variables
    let variables = prompts::prompt_variables(
        template,
        &opts.name,
        &config,
        opts.yes,
        opts.author.as_deref(),
        opts.org.as_deref(),
    )?;

    // Create project directory
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating directory {}", target_dir.display()))?;

    // Render template
    println!("{} Scaffolding project...", style("*").cyan().bold());
    let created_files = templates::render_template(template, &target_dir, &variables)?;

    // Write .fledge.toml for future `fledge update`
    update::write_project_meta(
        &target_dir,
        &template.name,
        None,
        None,
        template.manifest.template.version.as_deref(),
        &variables,
        &created_files,
    )?;

    // Git init
    if !opts.no_git {
        init_git(&target_dir)?;
    }

    // Post-create hooks (local templates are trusted); skip if required tools missing
    if !opts.no_install && reqs_ok {
        run_post_create_hooks(
            &template.manifest.hooks.post_create,
            &target_dir,
            false,
            opts.yes,
        )?;
    }

    // Print summary
    print_summary(&opts.name, &target_dir, &created_files, opts.no_git);

    Ok(())
}

fn run_remote(
    opts: InitOptions,
    remote_ref: &str,
    config: &Config,
    token: Option<&str>,
) -> Result<()> {
    let (owner, repo, subpath, git_ref) = crate::remote::parse_remote_ref(remote_ref);

    if opts.refresh {
        crate::remote::clear_cache(owner, repo)?;
    }

    let ref_display = git_ref.map(|r| format!("@{}", r)).unwrap_or_default();

    println!(
        "{} Fetching template from {}/{}{}{}...",
        style("*").cyan().bold(),
        owner,
        repo,
        subpath.map(|s| format!("/{}", s)).unwrap_or_default(),
        ref_display
    );

    let template_dir = crate::remote::resolve_template_dir(owner, repo, subpath, token, git_ref)?;

    // The remote dir might be a single template or a collection
    let mut found = Vec::new();
    if template_dir.join("template.toml").exists() {
        // Single template
        let content = std::fs::read_to_string(template_dir.join("template.toml"))?;
        let manifest: templates::TemplateManifest = toml::from_str(&content)?;
        found.push(templates::Template {
            name: manifest.template.name.clone(),
            description: manifest.template.description.clone(),
            path: template_dir.clone(),
            manifest,
        });
    } else {
        // Collection — discover templates within
        let extra = vec![template_dir.clone()];
        found = templates::discover_templates(&extra)?;
    }

    if found.is_empty() {
        bail!("No templates found in {}", remote_ref);
    }

    let template = if found.len() == 1 {
        &found[0]
    } else {
        let idx = prompts::select_template(&found)?;
        &found[idx]
    };

    println!(
        "{} Using template: {}",
        style("*").cyan().bold(),
        style(&template.name).green()
    );

    check_template_version(&template.manifest)?;
    let reqs_ok = check_template_requirements(&template.manifest, opts.yes)?;

    let target_dir = opts.output.join(&opts.name);
    if target_dir.exists() {
        bail!(
            "Directory '{}' already exists. Choose a different name or remove it first.",
            target_dir.display()
        );
    }

    if opts.dry_run {
        return print_dry_run(
            template,
            &target_dir,
            &template.manifest.hooks.post_create,
            opts.no_git,
            true,
        );
    }

    let variables = prompts::prompt_variables(
        template,
        &opts.name,
        config,
        opts.yes,
        opts.author.as_deref(),
        opts.org.as_deref(),
    )?;

    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating directory {}", target_dir.display()))?;

    println!("{} Scaffolding project...", style("*").cyan().bold());
    let created_files = templates::render_template(template, &target_dir, &variables)?;

    // Write .fledge.toml for future `fledge update`
    update::write_project_meta(
        &target_dir,
        &template.name,
        Some(remote_ref),
        git_ref,
        template.manifest.template.version.as_deref(),
        &variables,
        &created_files,
    )?;

    if !opts.no_git {
        init_git(&target_dir)?;
    }

    // Remote templates require confirmation before running hooks; skip if required tools missing
    if !opts.no_install && reqs_ok {
        run_post_create_hooks(
            &template.manifest.hooks.post_create,
            &target_dir,
            true,
            opts.yes,
        )?;
    }

    print_summary(&opts.name, &target_dir, &created_files, opts.no_git);

    Ok(())
}

fn print_summary(name: &str, target_dir: &Path, created_files: &[PathBuf], no_git: bool) {
    println!();
    println!(
        "{} Created {} in {}",
        style("✅").green().bold(),
        style(name).cyan().bold(),
        style(target_dir.display()).dim()
    );
    println!();
    for file in created_files {
        println!("  {}", style(file.display()).dim());
    }
    println!();
    println!("  {} files created", created_files.len());
    if !no_git {
        println!("  git repo initialized with initial commit");
    }
    println!();
    println!("  cd {} && get started!", style(name).cyan());
}

fn run_post_create_hooks(
    hooks: &[String],
    project_dir: &Path,
    is_remote: bool,
    auto_yes: bool,
) -> Result<()> {
    if hooks.is_empty() {
        return Ok(());
    }

    if is_remote && !auto_yes {
        println!();
        println!(
            "{} This template wants to run the following post-create hooks:",
            style("!").yellow().bold()
        );
        for cmd in hooks {
            println!("  {} {}", style("$").yellow(), cmd);
        }
        println!();

        let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Allow these commands to run?")
            .default(false)
            .interact()?;

        if !confirm {
            println!(
                "{} Skipped hooks. Run them manually or re-run with --yes.",
                style("*").cyan().bold()
            );
            return Ok(());
        }
    }

    println!("{} Running post-create hooks...", style("*").cyan().bold());

    for cmd in hooks {
        println!("  {} {}", style("$").dim(), style(cmd).dim());

        let status = std::process::Command::new("sh")
            .args(["-c", cmd])
            .current_dir(project_dir)
            .status()
            .with_context(|| format!("running hook: {}", cmd))?;

        if !status.success() {
            bail!(
                "Post-create hook failed (exit {}): {}",
                status.code().unwrap_or(-1),
                cmd
            );
        }
    }

    Ok(())
}

fn print_dry_run(
    template: &Template,
    target_dir: &Path,
    hooks: &[String],
    no_git: bool,
    is_remote: bool,
) -> Result<()> {
    println!();
    println!(
        "{} Dry run — nothing will be written",
        style("*").cyan().bold()
    );
    println!();
    println!("  Template:  {}", style(&template.name).green());
    println!("  Location:  {}", style(target_dir.display()).dim());
    println!("  Git init:  {}", if no_git { "no" } else { "yes" });

    if !template.manifest.template.requires.is_empty() {
        let (found, missing) = templates::check_requirements(&template.manifest.template.requires);
        print!("  Requires:  ");
        let parts: Vec<String> = found
            .iter()
            .map(|t| format!("{}", style(t).green()))
            .chain(
                missing
                    .iter()
                    .map(|t| format!("{}", style(format!("{t} (missing)")).red())),
            )
            .collect();
        println!("{}", parts.join(", "));
    }

    // List template files
    let files: Vec<_> = walkdir::WalkDir::new(&template.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() != "template.toml")
        .collect();

    println!();
    println!("  {} files would be created:", files.len());
    for entry in &files {
        if let Ok(rel) = entry.path().strip_prefix(&template.path) {
            println!("    {}", style(rel.display()).dim());
        }
    }

    if !hooks.is_empty() {
        println!();
        if is_remote {
            println!(
                "  {} Post-create hooks (requires confirmation):",
                style("!").yellow().bold()
            );
        } else {
            println!("  Post-create hooks:");
        }
        for cmd in hooks {
            println!("    {} {}", style("$").dim(), cmd);
        }
    }

    println!();
    Ok(())
}

fn check_template_version(manifest: &templates::TemplateManifest) -> Result<()> {
    if let Some(ref min_ver) = manifest.template.min_fledge_version {
        crate::versioning::check_fledge_version(min_ver)?;
    }
    Ok(())
}

/// Returns true if all requirements are met (hooks safe to run).
fn check_template_requirements(
    manifest: &templates::TemplateManifest,
    auto_yes: bool,
) -> Result<bool> {
    if manifest.template.requires.is_empty() {
        return Ok(true);
    }

    let (_, missing) = templates::check_requirements(&manifest.template.requires);
    if missing.is_empty() {
        return Ok(true);
    }

    println!(
        "\n{} This template requires tools not found on your PATH:",
        style("!").yellow().bold()
    );
    for tool in &missing {
        println!("  {} {}", style("missing:").yellow().bold(), tool);
    }
    println!();

    if auto_yes {
        println!(
            "{} Continuing anyway (--yes). Skipping post-create hooks.",
            style("*").cyan().bold()
        );
        return Ok(false);
    }

    let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Continue without these tools? (post-create hooks will be skipped)")
        .default(false)
        .interact()?;

    if !confirm {
        bail!(
            "Missing required tools: {}. Install them and try again.",
            missing.join(", ")
        );
    }

    Ok(false)
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

#[cfg(feature = "tui")]
pub fn init_git_for_tui(dir: &Path) -> Result<()> {
    init_git(dir)
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

    // Ensure git has a user configured (needed in CI / fresh environments)
    let has_user = std::process::Command::new("git")
        .args(["config", "user.name"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !has_user {
        std::process::Command::new("git")
            .args(["config", "user.name", "fledge"])
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .status()
            .ok();
        std::process::Command::new("git")
            .args(["config", "user.email", "fledge@localhost"])
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .status()
            .ok();
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

    #[test]
    fn run_post_create_hooks_runs_commands() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();

        let hooks = vec!["touch hook-ran.txt".to_string()];
        run_post_create_hooks(&hooks, &dir, false, false).unwrap();

        assert!(dir.join("hook-ran.txt").exists());
    }

    #[test]
    fn run_post_create_hooks_empty_is_noop() {
        let tmp = TempDir::new().unwrap();
        let result = run_post_create_hooks(&[], tmp.path(), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn run_post_create_hooks_failing_command_errors() {
        let tmp = TempDir::new().unwrap();
        let hooks = vec!["false".to_string()];
        let result = run_post_create_hooks(&hooks, tmp.path(), false, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Post-create hook failed"));
    }

    #[test]
    fn run_post_create_hooks_remote_with_yes_runs_without_prompt() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();

        let hooks = vec!["touch hook-ran.txt".to_string()];
        run_post_create_hooks(&hooks, &dir, true, true).unwrap();

        assert!(dir.join("hook-ran.txt").exists());
    }
}
