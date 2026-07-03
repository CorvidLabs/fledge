use anyhow::{bail, Context, Result};
use console::style;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::meta;
use crate::prompts;
use crate::templates::{self, Template};
use crate::trust;

/// JSON schema version for the `init` envelope. See lanes.rs for the per-command
/// rationale.
const INIT_SCHEMA: u32 = 1;

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
    /// Authorize post-create hook execution for **remote** templates without
    /// an interactive prompt. For local templates `yes` already covers this;
    /// remote templates require explicit hook trust because hooks run
    /// arbitrary shell commands from a third-party source.
    pub trust_hooks: bool,
    pub json: bool,
}

pub fn run(mut opts: InitOptions) -> Result<()> {
    if crate::utils::is_non_interactive() || opts.json {
        opts.yes = true;
    }
    if !opts.trust_hooks
        && std::env::var("FLEDGE_TRUST_HOOKS")
            .ok()
            .is_some_and(|v| crate::utils::is_truthy_env(&v))
    {
        opts.trust_hooks = true;
    }
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

    if !opts.json
        && opts.template.is_none()
        && config.template_repos().is_empty()
        && available.len() <= 2
    {
        println!(
            "{} Only built-in starter templates found. Add {} to your config or pass `-t <owner/repo>` to fetch a remote template.",
            style("tip:").yellow().bold(),
            style("CorvidLabs/fledge-templates").cyan(),
        );
    }

    // Resolve which template to use
    let template = resolve_template(&available, opts.template.as_deref())?;

    if !opts.json {
        println!(
            "{} Using template: {}",
            style("*").cyan().bold(),
            style(&template.name).green()
        );
    }

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
    if !opts.json {
        println!("{} Scaffolding project...", style("*").cyan().bold());
    }
    let mut created_files = templates::render_template(template, &target_dir, &variables)?;

    // Generate fledge.toml if the template didn't include one
    generate_fledge_toml_if_missing(&target_dir, &mut created_files)?;

    // Write .fledge/meta.toml for future `fledge update`
    meta::write_project_meta(
        &target_dir,
        &template.name,
        None,
        None,
        template.manifest.template.version.as_deref(),
        &variables,
        &created_files,
    )?;

    // Git init
    let git_initialized = if opts.no_git {
        false
    } else {
        init_git(&target_dir)?
    };

    let hooks_run = !opts.no_install && reqs_ok && !template.manifest.hooks.post_create.is_empty();
    // Post-create hooks for **local** templates: `--yes` is sufficient consent
    // because the template was authored by the user (or someone they pulled
    // into their own filesystem). Remote-template hooks take a different path
    // through `run_remote` with stricter consent rules.
    if !opts.no_install && reqs_ok {
        run_post_create_hooks(
            &template.manifest.hooks.post_create,
            &target_dir,
            opts.yes,
            opts.json,
            HookSource::Local,
        )?;
    }

    if opts.json {
        emit_init_envelope(
            &opts.name,
            &target_dir,
            template,
            None,
            &variables,
            &created_files,
            git_initialized,
            hooks_run,
        )?;
    } else {
        print_summary(&opts.name, &target_dir, &created_files, git_initialized);
    }

    Ok(())
}

/// Provenance of a template being rendered. Drives the consent rule used by
/// `run_post_create_hooks` and the hint text shown when hooks are skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookSource {
    /// Template lives on the user's filesystem (built-in starter or under a
    /// configured `extra_paths`). The user is presumed to have authored or
    /// vetted it.
    Local,
    /// Template was fetched from a remote GitHub repo. Hooks require explicit
    /// trust via `--trust-hooks` / `FLEDGE_TRUST_HOOKS`, **not** `--yes`.
    Remote,
}

#[allow(clippy::too_many_arguments)]
fn emit_init_envelope(
    name: &str,
    target_dir: &Path,
    template: &Template,
    remote_ref: Option<&str>,
    variables: &tera::Context,
    created_files: &[PathBuf],
    git_initialized: bool,
    hooks_run: bool,
) -> Result<()> {
    let result = crate::envelope::action(
        INIT_SCHEMA,
        "init",
        serde_json::json!({
            "project": {
                "name": name,
                "path": target_dir.display().to_string(),
            },
            "template": {
                "name": template.name,
                "source": remote_ref,
                "version": template.manifest.template.version,
            },
            "variables_used": variables.clone().into_json(),
            "files_created": created_files
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
            "git_initialized": git_initialized,
            "hooks_run": hooks_run,
        }),
    );
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn run_remote(
    opts: InitOptions,
    remote_ref: &str,
    config: &Config,
    token: Option<&str>,
) -> Result<()> {
    let (owner, repo, subpath, git_ref) = crate::remote::parse_remote_ref(remote_ref)?;

    if opts.refresh {
        crate::remote::clear_cache(owner, repo)?;
    }

    let ref_display = git_ref.map(|r| format!("@{}", r)).unwrap_or_default();

    if !opts.json {
        println!(
            "{} Fetching template from {}/{}{}{}...",
            style("*").cyan().bold(),
            owner,
            repo,
            subpath.map(|s| format!("/{}", s)).unwrap_or_default(),
            ref_display
        );
    }

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
            source: Some(remote_ref.to_string()),
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

    let tier = trust::determine_trust_tier(remote_ref);
    if !opts.json {
        println!(
            "{} Using template: {} [{}]",
            style("*").cyan().bold(),
            style(&template.name).green(),
            tier.styled_label()
        );
        if tier != trust::TrustTier::Official {
            println!(
                "  {} Templates can include arbitrary files and post-create hooks.",
                style("*").yellow()
            );
            println!(
                "  {} Only use templates from sources you trust.\n",
                style("*").yellow()
            );
        }
    }

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

    if !opts.json {
        println!("{} Scaffolding project...", style("*").cyan().bold());
    }
    let mut created_files = templates::render_template(template, &target_dir, &variables)?;

    // Generate fledge.toml if the template didn't include one
    generate_fledge_toml_if_missing(&target_dir, &mut created_files)?;

    // Write .fledge/meta.toml for future `fledge update`
    meta::write_project_meta(
        &target_dir,
        &template.name,
        Some(remote_ref),
        git_ref,
        template.manifest.template.version.as_deref(),
        &variables,
        &created_files,
    )?;

    let git_initialized = if opts.no_git {
        false
    } else {
        init_git(&target_dir)?
    };

    // Remote templates: `--yes` alone is **not** consent for arbitrary shell
    // execution from a third-party source. Require `--trust-hooks` (or
    // `FLEDGE_TRUST_HOOKS=1`) explicitly. In interactive mode the prompt
    // still fires as the fallback.
    let auto_yes_for_hooks = opts.trust_hooks;
    let hooks_run = !opts.no_install && reqs_ok && !template.manifest.hooks.post_create.is_empty();
    if !opts.no_install && reqs_ok {
        run_post_create_hooks(
            &template.manifest.hooks.post_create,
            &target_dir,
            auto_yes_for_hooks,
            opts.json,
            HookSource::Remote,
        )?;
    }

    if opts.json {
        emit_init_envelope(
            &opts.name,
            &target_dir,
            template,
            Some(remote_ref),
            &variables,
            &created_files,
            git_initialized,
            hooks_run,
        )?;
    } else {
        print_summary(&opts.name, &target_dir, &created_files, git_initialized);
    }

    Ok(())
}

fn print_summary(name: &str, target_dir: &Path, created_files: &[PathBuf], git_initialized: bool) {
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
    if git_initialized {
        println!("  git repo initialized with initial commit");
    }
    println!();
    println!("  cd {} && get started!", style(name).cyan());
}

fn run_post_create_hooks(
    hooks: &[String],
    project_dir: &Path,
    auto_yes: bool,
    quiet: bool,
    source: HookSource,
) -> Result<()> {
    if hooks.is_empty() {
        return Ok(());
    }

    let consent_flag = match source {
        HookSource::Local => "--yes",
        HookSource::Remote => "--trust-hooks",
    };

    if !auto_yes {
        if !quiet {
            println!();
            let header = match source {
                HookSource::Local => "This template wants to run the following post-create hooks:",
                HookSource::Remote => {
                    "This REMOTE template wants to run the following post-create hooks:"
                }
            };
            println!("{} {}", style("!").yellow().bold(), header);
            for cmd in hooks {
                println!("  {} {}", style("$").yellow(), cmd);
            }
            if source == HookSource::Remote {
                println!();
                println!(
                    "  {} These commands run with your shell privileges. Only allow if you trust the source.",
                    style("*").yellow()
                );
            }
            println!();
        }

        if !crate::utils::is_interactive() {
            if !quiet {
                println!(
                    "{} Skipped hooks (non-interactive). Re-run with {} to allow.",
                    style("*").cyan().bold(),
                    style(consent_flag).cyan()
                );
            }
            return Ok(());
        }

        let confirm = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Allow these commands to run?")
            .default(false)
            .interact()?;

        if !confirm {
            if !quiet {
                println!(
                    "{} Skipped hooks. Run them manually or re-run with {}.",
                    style("*").cyan().bold(),
                    style(consent_flag).cyan()
                );
            }
            return Ok(());
        }
    }

    if !quiet {
        println!("{} Running post-create hooks...", style("*").cyan().bold());
    }

    for cmd in hooks {
        if !quiet {
            println!("  {} {}", style("$").dim(), style(cmd).dim());
        }

        if cmd.trim().is_empty() {
            bail!("Empty post-create hook command");
        }

        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        let flag = if cfg!(windows) { "/C" } else { "-c" };

        let output = std::process::Command::new(shell)
            .args([flag, cmd])
            .current_dir(project_dir)
            .output()
            .with_context(|| format!("running hook: {}", cmd))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut detail = String::new();
            if !stdout.trim().is_empty() {
                detail.push_str(&format!("\n  stdout: {}", stdout.trim()));
            }
            if !stderr.trim().is_empty() {
                detail.push_str(&format!("\n  stderr: {}", stderr.trim()));
            }
            bail!(
                "Post-create hook failed (exit {}): {}{}",
                output.status.code().unwrap_or(-1),
                cmd,
                detail
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

    if auto_yes || !crate::utils::is_interactive() {
        println!(
            "{} Continuing anyway{}. Skipping post-create hooks.",
            style("*").cyan().bold(),
            if auto_yes {
                " (--yes)"
            } else {
                " (non-interactive)"
            }
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

fn init_git(dir: &Path) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("running git init")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git init failed: {}", stderr.trim());
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
    let add = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("running git add")?;
    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr);
        eprintln!(
            "{} git add failed, skipping initial commit: {}",
            style("!").yellow().bold(),
            stderr.trim()
        );
        return Ok(false);
    }

    // Only commit if something is staged. `git diff --cached --quiet` exits 0
    // when the index is empty (benign — e.g. a template with no files) and
    // non-zero when there are staged changes. This is locale-independent,
    // unlike parsing git's "nothing to commit" message.
    let staged = std::process::Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(dir)
        .status()
        .context("checking for staged changes")?;
    if staged.success() {
        // Nothing staged; `git init` succeeded, so report success without a commit.
        return Ok(true);
    }

    // Initial commit — any failure here (failing hook, missing identity, ...)
    // is real, since we already confirmed there are staged changes.
    let commit = std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit from fledge"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("running git commit")?;
    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        eprintln!(
            "{} git commit failed: {}",
            style("!").yellow().bold(),
            stderr.trim()
        );
        return Ok(false);
    }

    Ok(true)
}

fn generate_fledge_toml_if_missing(
    target_dir: &Path,
    created_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let fledge_toml = target_dir.join("fledge.toml");
    if fledge_toml.exists() {
        return Ok(());
    }

    let project_type = crate::run::detect_project_type(target_dir);
    let defaults = crate::run::task_defaults(project_type, target_dir);

    let content = format!(
        r#"# fledge.toml — project task definitions
# Docs: https://github.com/CorvidLabs/fledge#task-runner

[tasks]
{defaults}
"#
    );

    std::fs::write(&fledge_toml, content).context("writing fledge.toml")?;
    created_files.push(PathBuf::from("fledge.toml"));
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

    #[cfg(unix)]
    #[test]
    fn init_git_reports_false_when_commit_fails() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-project");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file.txt"), "hello").unwrap();

        // Pre-create the repo with a pre-commit hook that always fails.
        // git init is idempotent, so init_git re-inits and keeps the hook.
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let hooks = dir.join(".git/hooks");
        fs::create_dir_all(&hooks).unwrap();
        let hook = hooks.join("pre-commit");
        fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

        let initialized = init_git(&dir).unwrap();
        assert!(
            !initialized,
            "a failed commit must report git_initialized = false"
        );
    }

    #[test]
    fn run_post_create_hooks_runs_commands() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();

        let hooks = vec!["touch hook-ran.txt".to_string()];
        run_post_create_hooks(&hooks, &dir, true, false, HookSource::Local).unwrap();

        assert!(dir.join("hook-ran.txt").exists());
    }

    #[test]
    fn run_post_create_hooks_empty_is_noop() {
        let tmp = TempDir::new().unwrap();
        let result = run_post_create_hooks(&[], tmp.path(), true, false, HookSource::Local);
        assert!(result.is_ok());
    }

    #[test]
    fn run_post_create_hooks_failing_command_errors() {
        let tmp = TempDir::new().unwrap();
        let hooks = vec!["false".to_string()];
        let result = run_post_create_hooks(&hooks, tmp.path(), true, false, HookSource::Local);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Post-create hook failed"));
    }

    #[test]
    fn run_post_create_hooks_with_yes_runs_without_prompt() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();

        let hooks = vec!["touch hook-ran.txt".to_string()];
        run_post_create_hooks(&hooks, &dir, true, false, HookSource::Local).unwrap();

        assert!(dir.join("hook-ran.txt").exists());
    }

    #[test]
    fn run_post_create_hooks_remote_skipped_when_not_trusted_in_non_interactive() {
        // Locks the D4 contract: a remote-template hook with `auto_yes: false`
        // (i.e. `--trust-hooks` not passed) is skipped in non-interactive mode,
        // not auto-run. The hint text guides the user to `--trust-hooks` rather
        // than `--yes`.
        let _guard = crate::test_support::NonInteractiveGuard::new(true);
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();

        let hooks = vec!["touch hook-ran.txt".to_string()];
        // auto_yes = false simulates a user who passed `--yes` (which doesn't
        // grant remote hooks) but not `--trust-hooks`. The remote call site in
        // `run_remote` would compute auto_yes = opts.trust_hooks = false here.
        let result = run_post_create_hooks(&hooks, &dir, false, true, HookSource::Remote);
        assert!(result.is_ok(), "should skip cleanly, not error");
        assert!(
            !dir.join("hook-ran.txt").exists(),
            "remote hook should NOT execute without --trust-hooks in non-interactive mode"
        );
    }

    #[test]
    fn run_post_create_hooks_remote_runs_with_trust_hooks() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();

        let hooks = vec!["touch hook-ran.txt".to_string()];
        // auto_yes = true simulates `--trust-hooks` having been passed.
        run_post_create_hooks(&hooks, &dir, true, true, HookSource::Remote).unwrap();
        assert!(dir.join("hook-ran.txt").exists());
    }
}
