use anyhow::{bail, Context, Result};
use console::style;
use std::fmt;
use std::path::Path;
use std::process::Command;

use crate::config::Config;
use crate::llm::{self, ProviderOverride};
use crate::spec;

#[derive(Debug, Clone, Default)]
pub enum ReviewFormat {
    #[default]
    Summary,
    Checklist,
    Inline,
}

impl fmt::Display for ReviewFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewFormat::Summary => write!(f, "summary"),
            ReviewFormat::Checklist => write!(f, "checklist"),
            ReviewFormat::Inline => write!(f, "inline"),
        }
    }
}

impl std::str::FromStr for ReviewFormat {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "summary" => Ok(ReviewFormat::Summary),
            "checklist" => Ok(ReviewFormat::Checklist),
            "inline" => Ok(ReviewFormat::Inline),
            other => Err(format!(
                "unknown review format '{}' (expected: summary, checklist, inline)",
                other
            )),
        }
    }
}

pub struct ReviewOptions {
    pub base: Option<String>,
    pub file: Option<String>,
    pub json: bool,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub format: ReviewFormat,
    pub with_specs: Vec<String>,
    pub no_auto_specs: bool,
    pub provider: Option<String>,
}

pub fn run(options: ReviewOptions) -> Result<()> {
    crate::github::ensure_git_repo()?;

    let base = match options.base {
        Some(b) => b,
        None => default_branch()?,
    };

    let diff = get_diff(&base, options.file.as_deref())?;

    if diff.is_empty() {
        bail!("No changes to review against '{}'.", base);
    }

    let diff_stats = get_diff_stats(&base, options.file.as_deref())?;
    let changed_files = get_changed_files(&base, options.file.as_deref())?;

    if !options.json && !diff_stats.is_empty() {
        println!("{}\n", style(&diff_stats).dim());
    }

    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let spec_context = build_spec_context(
        &root,
        &changed_files,
        &options.with_specs,
        options.no_auto_specs,
    )?;

    if !options.json {
        if let Some(names) = spec_context.as_ref().map(|(names, _)| names.clone()) {
            if !names.is_empty() {
                println!(
                    "{} {}",
                    style("Spec context:").dim(),
                    style(names.join(", ")).cyan()
                );
                println!();
            }
        }
    }

    let prompt = build_prompt(
        &diff,
        &options.format,
        options.prompt.as_deref(),
        spec_context.as_ref().map(|(_, body)| body.as_str()),
    );

    let config = Config::load().context("loading config")?;
    let provider = llm::build_provider(
        &config,
        &ProviderOverride {
            provider: options.provider.clone(),
            model: options.model.clone(),
        },
    )?;

    let sp = crate::spinner::Spinner::start(&format!(
        "Reviewing changes against {} [{}]:",
        &base,
        llm::describe(&*provider)
    ));
    // Finish spinner before surfacing provider errors so the terminal state
    // is clean when `bail!` fires.
    let answer = provider.invoke(&prompt);
    sp.finish();
    println!();
    let answer = answer?;

    if options.json {
        let spec_names = spec_context
            .as_ref()
            .map(|(names, _)| names.clone())
            .unwrap_or_default();
        let response = serde_json::json!({
            "base": base,
            "file": options.file,
            "diff_stats": diff_stats,
            "spec_context": spec_names,
            "review": answer.trim(),
            "provider": provider.kind().as_str(),
            "model": provider.model_name(),
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("{answer}");
    }

    Ok(())
}

/// Returns `(module_names, prompt_body)` for the spec context to include, or
/// `None` if no specs are to be included.
fn build_spec_context(
    root: &Path,
    changed_files: &[String],
    with_specs: &[String],
    no_auto_specs: bool,
) -> Result<Option<(Vec<String>, String)>> {
    let mut names: Vec<String> = Vec::new();

    if !no_auto_specs {
        // Auto-detect: match by frontmatter files: and by specs/<name>/ prefix.
        // Silent fallback to empty list if the project isn't spec-tracked.
        if let Ok(matched) = spec::specs_for_changed_files(root, changed_files) {
            names.extend(matched);
        }
    }

    for raw in with_specs {
        for part in raw.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                names.push(trimmed.to_string());
            }
        }
    }

    names.sort();
    names.dedup();

    if names.is_empty() {
        return Ok(None);
    }

    let mut body = String::new();
    for name in &names {
        let bundle = spec::load_module_bundle(root, name)
            .with_context(|| format!("loading spec bundle for '{name}'"))?;
        body.push_str(&bundle);
    }

    Ok(Some((names, body)))
}

fn build_prompt(
    diff: &str,
    format: &ReviewFormat,
    custom_prompt: Option<&str>,
    spec_context: Option<&str>,
) -> String {
    let format_instruction = match format {
        ReviewFormat::Summary => {
            "Be concise. Use markdown formatting. Only comment on things worth changing.\n\
             If the code looks good, say so briefly."
                .to_string()
        }
        ReviewFormat::Checklist => {
            "Format your review as a markdown checklist with - [ ] for issues found and - [x] for areas that look good."
                .to_string()
        }
        ReviewFormat::Inline => {
            "For each file in the diff, provide inline comments in the format: `file:line - comment`. Group by file."
                .to_string()
        }
    };

    let custom_section = match custom_prompt {
        Some(p) => format!("\n\nAdditional review focus: {p}"),
        None => String::new(),
    };

    let context_section = match spec_context {
        Some(ctx) => format!(
            "\n\nBackground context — these are the formal specs for the modules touched by the diff below. \
            They describe *what the modules are supposed to do*. Use them to interpret the changes.\n\n\
            CRITICAL: your review must cover **only** the diff. Do NOT suggest changes to code that wasn't \
            modified. Do NOT critique or review the specs themselves — they are context only. If the diff \
            contradicts a spec invariant, call that out as a bug in the diff.\n\n\
            {ctx}\n"
        ),
        None => String::new(),
    };

    format!(
        "You are a senior code reviewer. Review the following git diff and provide actionable feedback.\n\
        Focus on:\n\
        - Bugs and logic errors\n\
        - Security issues\n\
        - Performance concerns\n\
        - Code clarity and maintainability\n\
        {context_section}\n\
        {format_instruction}{custom_section}\n\
        \n\
        ```diff\n{diff}\n```"
    )
}

fn default_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(name) = branch.strip_prefix("origin/") {
            return Ok(name.to_string());
        }
        return Ok(branch);
    }

    for candidate in &["main", "master"] {
        let check = Command::new("git")
            .args(["rev-parse", "--verify", candidate])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        if check.success() {
            return Ok(candidate.to_string());
        }
    }

    Ok("main".to_string())
}

fn get_diff(base: &str, file: Option<&str>) -> Result<String> {
    let mut args = vec!["diff", base];
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }

    let output = Command::new("git").args(&args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn get_diff_stats(base: &str, file: Option<&str>) -> Result<String> {
    let mut args = vec!["diff", "--stat", base];
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }

    let output = Command::new("git").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_changed_files(base: &str, file: Option<&str>) -> Result<Vec<String>> {
    let mut args = vec!["diff", "--name-only", base];
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }
    let output = Command::new("git").args(&args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff --name-only failed: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_contains_diff() {
        let prompt = build_prompt(
            "+ added line\n- removed line",
            &ReviewFormat::Summary,
            None,
            None,
        );
        assert!(prompt.contains("+ added line"));
        assert!(prompt.contains("- removed line"));
        assert!(prompt.contains("```diff"));
    }

    #[test]
    fn build_prompt_includes_review_criteria() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None, None);
        assert!(prompt.contains("Bugs and logic errors"));
        assert!(prompt.contains("Security issues"));
        assert!(prompt.contains("Performance concerns"));
        assert!(prompt.contains("Code clarity"));
    }

    #[test]
    fn build_prompt_summary_format() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None, None);
        assert!(prompt.contains("Be concise"));
        assert!(prompt.contains("Use markdown formatting"));
    }

    #[test]
    fn build_prompt_checklist_format() {
        let prompt = build_prompt("some diff", &ReviewFormat::Checklist, None, None);
        assert!(prompt.contains("markdown checklist"));
        assert!(prompt.contains("- [ ]"));
        assert!(prompt.contains("- [x]"));
    }

    #[test]
    fn build_prompt_inline_format() {
        let prompt = build_prompt("some diff", &ReviewFormat::Inline, None, None);
        assert!(prompt.contains("file:line - comment"));
        assert!(prompt.contains("Group by file"));
    }

    #[test]
    fn build_prompt_with_custom_prompt() {
        let prompt = build_prompt(
            "some diff",
            &ReviewFormat::Summary,
            Some("Focus on security vulnerabilities"),
            None,
        );
        assert!(prompt.contains("Additional review focus: Focus on security vulnerabilities"));
    }

    #[test]
    fn build_prompt_without_custom_prompt() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None, None);
        assert!(!prompt.contains("Additional review focus"));
    }

    #[test]
    fn build_prompt_custom_prompt_with_checklist_format() {
        let prompt = build_prompt(
            "some diff",
            &ReviewFormat::Checklist,
            Some("Check for performance issues"),
            None,
        );
        assert!(prompt.contains("markdown checklist"));
        assert!(prompt.contains("Additional review focus: Check for performance issues"));
    }

    #[test]
    fn build_prompt_includes_spec_context_when_provided() {
        let ctx = "## Spec bundle: trust\n\ntrust spec body";
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None, Some(ctx));
        assert!(prompt.contains("Background context"));
        assert!(prompt.contains("trust spec body"));
    }

    #[test]
    fn build_prompt_spec_context_constrains_scope() {
        let ctx = "## Spec bundle: trust\n\ntrust spec body";
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None, Some(ctx));
        // The spec-context block must tell Claude the review target is the diff, not the specs.
        assert!(prompt.contains("CRITICAL"));
        assert!(prompt.contains("context only"));
        assert!(prompt.contains("Do NOT suggest changes to code that wasn't"));
        assert!(prompt.contains("Do NOT critique or review the specs"));
    }

    #[test]
    fn build_prompt_omits_spec_block_when_none() {
        let prompt = build_prompt("some diff", &ReviewFormat::Summary, None, None);
        assert!(!prompt.contains("Background context"));
    }

    #[test]
    fn review_format_from_str() {
        assert!(matches!(
            "summary".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Summary
        ));
        assert!(matches!(
            "checklist".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Checklist
        ));
        assert!(matches!(
            "inline".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Inline
        ));
        assert!(matches!(
            "SUMMARY".parse::<ReviewFormat>().unwrap(),
            ReviewFormat::Summary
        ));
        assert!("unknown".parse::<ReviewFormat>().is_err());
    }

    #[test]
    fn review_format_display() {
        assert_eq!(ReviewFormat::Summary.to_string(), "summary");
        assert_eq!(ReviewFormat::Checklist.to_string(), "checklist");
        assert_eq!(ReviewFormat::Inline.to_string(), "inline");
    }

    fn default_review_options() -> ReviewOptions {
        ReviewOptions {
            base: None,
            file: None,
            json: false,
            model: None,
            prompt: None,
            format: ReviewFormat::Summary,
            with_specs: Vec::new(),
            no_auto_specs: false,
            provider: None,
        }
    }

    #[test]
    fn review_options_defaults() {
        let opts = default_review_options();
        assert!(opts.base.is_none());
        assert!(opts.file.is_none());
        assert!(!opts.json);
        assert!(opts.model.is_none());
        assert!(opts.prompt.is_none());
        assert!(matches!(opts.format, ReviewFormat::Summary));
        assert!(opts.with_specs.is_empty());
        assert!(!opts.no_auto_specs);
        assert!(opts.provider.is_none());
    }

    #[test]
    fn review_options_with_base() {
        let opts = ReviewOptions {
            base: Some("develop".to_string()),
            json: true,
            ..default_review_options()
        };
        assert_eq!(opts.base.unwrap(), "develop");
        assert!(opts.json);
    }

    #[test]
    fn review_options_with_file() {
        let opts = ReviewOptions {
            file: Some("src/main.rs".to_string()),
            ..default_review_options()
        };
        assert_eq!(opts.file.unwrap(), "src/main.rs");
    }

    #[test]
    fn review_options_with_all_new_fields() {
        let opts = ReviewOptions {
            model: Some("opus".to_string()),
            prompt: Some("Focus on security".to_string()),
            format: ReviewFormat::Checklist,
            with_specs: vec!["trust".to_string()],
            no_auto_specs: true,
            provider: Some("ollama".to_string()),
            ..default_review_options()
        };
        assert_eq!(opts.model.unwrap(), "opus");
        assert_eq!(opts.prompt.unwrap(), "Focus on security");
        assert!(matches!(opts.format, ReviewFormat::Checklist));
        assert_eq!(opts.with_specs, vec!["trust"]);
        assert!(opts.no_auto_specs);
        assert_eq!(opts.provider.unwrap(), "ollama");
    }

    #[test]
    fn build_spec_context_returns_none_when_no_specs_requested_and_disabled() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let ctx = build_spec_context(tmp.path(), &["some/file.rs".to_string()], &[], true).unwrap();
        assert!(ctx.is_none());
    }

    #[test]
    fn build_spec_context_combines_auto_and_explicit() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let specsync = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync).unwrap();
        fs::write(
            specsync.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();
        for (name, file) in [("trust", "src/trust.rs"), ("work", "src/work.rs")] {
            let dir = tmp.path().join(format!("specs/{name}"));
            fs::create_dir_all(&dir).unwrap();
            let spec = format!(
                "---\nmodule: {name}\nversion: 1\nstatus: active\nfiles:\n  - {file}\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n"
            );
            fs::write(dir.join(format!("{name}.spec.md")), spec).unwrap();
        }

        // auto-detect will match trust via src/trust.rs; --with-specs adds work
        let changed = vec!["src/trust.rs".to_string()];
        let with = vec!["work".to_string()];
        let (names, body) = build_spec_context(tmp.path(), &changed, &with, false)
            .unwrap()
            .unwrap();
        assert_eq!(names, vec!["trust", "work"]);
        assert!(body.contains("## Spec bundle: trust"));
        assert!(body.contains("## Spec bundle: work"));
    }

    #[test]
    fn build_spec_context_no_auto_specs_skips_autodetect() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let specsync = tmp.path().join(".specsync");
        fs::create_dir_all(&specsync).unwrap();
        fs::write(
            specsync.join("config.toml"),
            "specs_dir = \"specs\"\nrequired_sections = []\n",
        )
        .unwrap();
        let dir = tmp.path().join("specs/trust");
        fs::create_dir_all(&dir).unwrap();
        let spec = "---\nmodule: trust\nversion: 1\nstatus: active\nfiles:\n  - src/trust.rs\n\ndb_tables: []\ndepends_on: []\n---\n\n## Purpose\n\nP.\n";
        fs::write(dir.join("trust.spec.md"), spec).unwrap();

        // src/trust.rs is in diff, but --no-auto-specs should prevent auto-include
        let changed = vec!["src/trust.rs".to_string()];
        let ctx = build_spec_context(tmp.path(), &changed, &[], true).unwrap();
        assert!(ctx.is_none());
    }
}
