use anyhow::{bail, Context, Result};
use console::style;
use std::fmt;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::config::Config;
use crate::llm::{self, ProviderKind, ProviderOverride};
use crate::spec;

/// JSON schema version for the `review` envelope. See lanes.rs for the
/// per-command rationale.
const REVIEW_SCHEMA: u32 = 1;

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
    pub with_model: Vec<String>,
    pub no_active: bool,
}

/// One slot in a multi-model review panel. `model` is `None` when the user
/// passes a bare provider name like `--with-model ollama`, in which case we
/// fall back to the provider's active config selection.
#[derive(Debug, Clone, PartialEq)]
struct ModelRef {
    provider: String,
    model: Option<String>,
}

/// Parse a `provider[:model]` ref. Splits on the FIRST colon only so that
/// model names with colons (`gpt-oss:120b-cloud`, `qwen3-coder:480b-cloud`)
/// round-trip cleanly. The provider half is validated against
/// `ProviderKind::parse` so typos like `--with-model claud:opus` fail at
/// parse time, not after the spinner has been spinning for 30s.
fn parse_model_ref(s: &str) -> Result<ModelRef> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        bail!("empty --with-model entry");
    }
    let (provider_raw, model_raw) = match trimmed.split_once(':') {
        Some((p, m)) => (p.trim(), m.trim()),
        None => (trimmed, ""),
    };
    if provider_raw.is_empty() {
        bail!("missing provider in '{trimmed}' (expected provider[:model])");
    }
    // Validate the provider against the known set; bubble up the parse error
    // so the user gets the same message they'd get from `--provider`.
    let provider = ProviderKind::parse(provider_raw)?.as_str().to_string();
    let model = if model_raw.is_empty() {
        None
    } else {
        Some(model_raw.to_string())
    };
    Ok(ModelRef { provider, model })
}

/// Result of one review in the panel. `outcome` is `Err` when the provider
/// failed (timeout, HTTP error, etc.) — we capture instead of bailing so a
/// single broken model doesn't poison the whole panel run.
struct PanelResult {
    provider_kind: String,
    model_name: Option<String>,
    elapsed_seconds: f64,
    outcome: Result<String>,
}

pub fn run(options: ReviewOptions) -> Result<()> {
    crate::github::ensure_git_repo()?;

    let base = match &options.base {
        Some(b) => b.clone(),
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

    // Build the panel: optionally the active config (one slot honoring
    // --provider/--model overrides), then each --with-model entry. Order is
    // preserved end-to-end so output matches what the user typed.
    let overrides = resolve_overrides(&options)?;

    // Build all providers up front so config errors fail fast and are
    // attributed to the right slot, before we kick off any threads.
    let providers: Vec<Box<dyn llm::LlmProvider>> = overrides
        .iter()
        .enumerate()
        .map(|(i, ov)| {
            llm::build_provider(&config, ov).with_context(|| format!("review panel slot {i}"))
        })
        .collect::<Result<_>>()?;

    let panel_size = providers.len();
    let panel_summary = providers
        .iter()
        .map(|p| llm::describe(&**p))
        .collect::<Vec<_>>()
        .join(", ");

    let spinner_msg = if panel_size == 1 {
        format!("Reviewing changes against {} [{}]:", base, panel_summary)
    } else {
        format!(
            "Reviewing changes against {} across {} models [{}]:",
            base, panel_size, panel_summary
        )
    };
    let sp = crate::spinner::Spinner::start(&spinner_msg);

    let results = run_panel(providers, prompt)?;

    sp.finish();
    println!();

    if options.json {
        let spec_names = spec_context
            .as_ref()
            .map(|(names, _)| names.clone())
            .unwrap_or_default();
        let response = build_review_envelope(
            &base,
            options.file.as_deref(),
            &diff_stats,
            &spec_names,
            &results,
        );
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else if results.len() == 1 {
        // Preserve the v0.14 single-model output shape exactly.
        match &results[0].outcome {
            Ok(answer) => println!("{answer}"),
            Err(e) => bail!("{e}"),
        }
    } else {
        print_panel_human(&results);
    }

    Ok(())
}

/// Build the ordered provider-override list for the review panel: the active
/// config slot (unless `--no-active`), then each `--with-model` entry
/// (comma-split, `provider[:model]` parsed and validated). Order is preserved
/// so output matches what the user typed. Errors on an empty panel or a bad
/// model ref. Pure — no config load, no network.
fn resolve_overrides(options: &ReviewOptions) -> Result<Vec<ProviderOverride>> {
    let mut overrides: Vec<ProviderOverride> = Vec::new();
    if !options.no_active {
        overrides.push(ProviderOverride {
            provider: options.provider.clone(),
            model: options.model.clone(),
        });
    }
    for raw in &options.with_model {
        for part in raw.split(',') {
            let parsed = parse_model_ref(part)?;
            overrides.push(ProviderOverride {
                provider: Some(parsed.provider),
                model: parsed.model,
            });
        }
    }
    if overrides.is_empty() {
        bail!(
            "Empty review panel — pass --with-model <provider[:model]> or omit --no-active so the active config is included."
        );
    }
    Ok(overrides)
}

/// Run every provider in the panel concurrently against the same prompt,
/// returning results in panel order. Each slot captures its own outcome (in
/// `PanelResult::outcome`) so one provider's failure doesn't abort the others.
fn run_panel(
    providers: Vec<Box<dyn llm::LlmProvider>>,
    prompt: String,
) -> Result<Vec<PanelResult>> {
    let prompt_arc = Arc::new(prompt);
    let mut handles = Vec::with_capacity(providers.len());
    for (idx, provider) in providers.into_iter().enumerate() {
        let prompt_clone = Arc::clone(&prompt_arc);
        let handle = thread::spawn(move || {
            let kind = provider.kind().as_str().to_string();
            let model = provider.model_name().map(|s| s.to_string());
            let start = Instant::now();
            let outcome = provider.invoke(&prompt_clone);
            let elapsed = start.elapsed().as_secs_f64();
            (
                idx,
                PanelResult {
                    provider_kind: kind,
                    model_name: model,
                    elapsed_seconds: elapsed,
                    outcome,
                },
            )
        });
        handles.push(handle);
    }
    let mut indexed: Vec<(usize, PanelResult)> = handles
        .into_iter()
        .map(|h| {
            h.join().map_err(|e| {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                anyhow::anyhow!("review thread panicked: {}", msg)
            })
        })
        .collect::<Result<_>>()?;
    indexed.sort_by_key(|(i, _)| *i);
    Ok(indexed.into_iter().map(|(_, r)| r).collect())
}

/// The per-result JSON object (`provider`/`model`/`review`|`error`) shared by
/// the `reviews[]` array and, for single-model runs, the legacy top-level
/// fields. `include_elapsed` adds the rounded `elapsed_seconds` (array form
/// only; the legacy top-level fields omit it).
fn insert_result_fields(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    r: &PanelResult,
    include_elapsed: bool,
) {
    obj.insert("provider".into(), r.provider_kind.clone().into());
    obj.insert(
        "model".into(),
        match &r.model_name {
            Some(m) => serde_json::Value::String(m.clone()),
            None => serde_json::Value::Null,
        },
    );
    if include_elapsed {
        obj.insert(
            "elapsed_seconds".into(),
            serde_json::Number::from_f64((r.elapsed_seconds * 100.0).round() / 100.0)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    match &r.outcome {
        Ok(answer) => {
            obj.insert("review".into(), answer.trim().to_string().into());
        }
        Err(e) => {
            obj.insert("error".into(), e.to_string().into());
        }
    }
}

/// Assemble the `review` JSON envelope from the panel results. Single-model
/// runs additionally carry top-level `provider`/`model`/`review`|`error` for
/// backward compatibility with pre-panel scripts. Pure.
fn build_review_envelope(
    base: &str,
    file: Option<&str>,
    diff_stats: &str,
    spec_names: &[String],
    results: &[PanelResult],
) -> serde_json::Value {
    let reviews_json: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let mut obj = serde_json::Map::new();
            insert_result_fields(&mut obj, r, true);
            serde_json::Value::Object(obj)
        })
        .collect();
    let mut response = crate::envelope::action(
        REVIEW_SCHEMA,
        "review",
        serde_json::json!({
            "base": base,
            "file": file,
            "diff_stats": diff_stats,
            "spec_context": spec_names,
            "reviews": reviews_json,
        }),
    );
    // Single-model invocations keep the legacy top-level `review` / `provider`
    // / `model` fields so existing scripts don't break.
    if results.len() == 1 {
        let obj = response.as_object_mut().expect("json object");
        insert_result_fields(obj, &results[0], false);
    }
    response
}

/// Print the multi-model panel results as banner-separated blocks, plus a
/// trailing note if any slot failed.
fn print_panel_human(results: &[PanelResult]) {
    for r in results {
        let label = match &r.model_name {
            Some(m) => format!("{} ({})", r.provider_kind, m),
            None => r.provider_kind.clone(),
        };
        let header = format!(" {} — {:.1}s ", label, r.elapsed_seconds);
        // Box the header in a banner that scales with the label width
        // so it stays visually distinct between dense markdown blocks.
        let bar = "═".repeat(60);
        println!();
        println!("{}", style(&bar).cyan());
        println!("{}", style(&header).bold().cyan());
        println!("{}", style(&bar).cyan());
        println!();
        match &r.outcome {
            Ok(answer) => println!("{}", answer.trim()),
            Err(e) => println!("{} {}", style("error:").red().bold(), e),
        }
    }
    let failures = results.iter().filter(|r| r.outcome.is_err()).count();
    if failures > 0 {
        println!();
        println!(
            "{} {}/{} models failed — see error blocks above. Successful reviews are unaffected.",
            style("⚠️").yellow(),
            failures,
            results.len()
        );
    }
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
    if base.starts_with('-') {
        bail!("Invalid base revision: cannot start with '-'");
    }
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
    if base.starts_with('-') {
        bail!("Invalid base revision: cannot start with '-'");
    }
    let mut args = vec!["diff", "--stat", base];
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }

    let output = Command::new("git").args(&args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_changed_files(base: &str, file: Option<&str>) -> Result<Vec<String>> {
    if base.starts_with('-') {
        bail!("Invalid base revision: cannot start with '-'");
    }
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
            with_model: Vec::new(),
            no_active: false,
        }
    }

    #[test]
    fn parse_model_ref_provider_only() {
        let r = parse_model_ref("anthropic").unwrap();
        assert_eq!(r.provider, "anthropic");
        assert_eq!(r.model, None);
    }

    #[test]
    fn parse_model_ref_provider_and_model() {
        let r = parse_model_ref("anthropic:claude-opus-4-8").unwrap();
        assert_eq!(r.provider, "anthropic");
        assert_eq!(r.model.as_deref(), Some("claude-opus-4-8"));
    }

    #[test]
    fn parse_model_ref_claude_alias_normalizes_to_anthropic() {
        let r = parse_model_ref("claude:claude-sonnet-4-6").unwrap();
        assert_eq!(r.provider, "anthropic");
        assert_eq!(r.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn parse_model_ref_model_with_colons() {
        // Ollama cloud model names contain colons — must round-trip cleanly.
        let r = parse_model_ref("ollama:gpt-oss:120b-cloud").unwrap();
        assert_eq!(r.provider, "ollama");
        assert_eq!(r.model.as_deref(), Some("gpt-oss:120b-cloud"));
    }

    #[test]
    fn parse_model_ref_qwen_three_segment() {
        let r = parse_model_ref("ollama:qwen3-coder:480b-cloud").unwrap();
        assert_eq!(r.provider, "ollama");
        assert_eq!(r.model.as_deref(), Some("qwen3-coder:480b-cloud"));
    }

    #[test]
    fn parse_model_ref_trims_whitespace() {
        let r = parse_model_ref("  ollama  :  gpt-oss:120b-cloud  ").unwrap();
        assert_eq!(r.provider, "ollama");
        assert_eq!(r.model.as_deref(), Some("gpt-oss:120b-cloud"));
    }

    #[test]
    fn parse_model_ref_empty_model_after_colon_means_active() {
        // `ollama:` should mean "use ollama's active config", same as bare `ollama`.
        let r = parse_model_ref("ollama:").unwrap();
        assert_eq!(r.provider, "ollama");
        assert_eq!(r.model, None);
    }

    #[test]
    fn parse_model_ref_rejects_unknown_provider() {
        let err = parse_model_ref("gpt:4").unwrap_err().to_string();
        assert!(
            err.to_lowercase().contains("provider") || err.contains("gpt"),
            "expected provider-rejection error, got: {err}"
        );
    }

    #[test]
    fn parse_model_ref_rejects_empty_input() {
        assert!(parse_model_ref("").is_err());
        assert!(parse_model_ref("   ").is_err());
    }

    #[test]
    fn parse_model_ref_rejects_missing_provider() {
        // ":opus" has no provider half — must error.
        assert!(parse_model_ref(":opus").is_err());
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

    // ── resolve_overrides ──────────────────────────────────────────────────

    #[test]
    fn resolve_overrides_default_is_single_active_slot() {
        let ov = resolve_overrides(&default_review_options()).unwrap();
        assert_eq!(ov.len(), 1);
        assert!(ov[0].provider.is_none());
        assert!(ov[0].model.is_none());
    }

    #[test]
    fn resolve_overrides_honors_active_provider_and_model() {
        let mut opts = default_review_options();
        opts.provider = Some("anthropic".to_string());
        opts.model = Some("claude-x".to_string());
        let ov = resolve_overrides(&opts).unwrap();
        assert_eq!(ov.len(), 1);
        assert_eq!(ov[0].provider.as_deref(), Some("anthropic"));
        assert_eq!(ov[0].model.as_deref(), Some("claude-x"));
    }

    #[test]
    fn resolve_overrides_no_active_without_models_errors() {
        let mut opts = default_review_options();
        opts.no_active = true;
        assert!(resolve_overrides(&opts).is_err());
    }

    #[test]
    fn resolve_overrides_comma_splits_with_model_in_order() {
        let mut opts = default_review_options();
        opts.no_active = true;
        opts.with_model = vec!["anthropic:opus,openai:gpt-x".to_string()];
        let ov = resolve_overrides(&opts).unwrap();
        assert_eq!(ov.len(), 2);
        assert_eq!(ov[0].provider.as_deref(), Some("anthropic"));
        assert_eq!(ov[0].model.as_deref(), Some("opus"));
        assert_eq!(ov[1].provider.as_deref(), Some("openai"));
        assert_eq!(ov[1].model.as_deref(), Some("gpt-x"));
    }

    #[test]
    fn resolve_overrides_active_then_with_model_bare_provider() {
        let mut opts = default_review_options();
        opts.with_model = vec!["ollama".to_string()];
        let ov = resolve_overrides(&opts).unwrap();
        assert_eq!(ov.len(), 2);
        assert!(ov[0].provider.is_none()); // active slot
        assert_eq!(ov[1].provider.as_deref(), Some("ollama"));
        assert!(ov[1].model.is_none()); // bare provider → active model
    }

    #[test]
    fn resolve_overrides_rejects_unknown_provider() {
        let mut opts = default_review_options();
        opts.with_model = vec!["notaprovider:x".to_string()];
        assert!(resolve_overrides(&opts).is_err());
    }

    // ── build_review_envelope ──────────────────────────────────────────────

    fn panel_ok(provider: &str, model: Option<&str>, elapsed: f64, review: &str) -> PanelResult {
        PanelResult {
            provider_kind: provider.to_string(),
            model_name: model.map(|s| s.to_string()),
            elapsed_seconds: elapsed,
            outcome: Ok(review.to_string()),
        }
    }

    fn panel_err(provider: &str, model: Option<&str>, elapsed: f64, err: &str) -> PanelResult {
        PanelResult {
            provider_kind: provider.to_string(),
            model_name: model.map(|s| s.to_string()),
            elapsed_seconds: elapsed,
            outcome: Err(anyhow::anyhow!("{}", err)),
        }
    }

    #[test]
    fn review_envelope_single_model_has_legacy_top_level_fields() {
        let results = vec![panel_ok(
            "anthropic",
            Some("claude-x"),
            1.234,
            "  looks good  ",
        )];
        let env = build_review_envelope(
            "main",
            Some("src/x.rs"),
            "1 file",
            &["specA".to_string()],
            &results,
        );
        assert_eq!(env["action"], "review");
        assert_eq!(env["base"], "main");
        assert_eq!(env["file"], "src/x.rs");
        assert_eq!(env["diff_stats"], "1 file");
        assert_eq!(env["spec_context"][0], "specA");
        let reviews = env["reviews"].as_array().unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0]["provider"], "anthropic");
        assert_eq!(reviews[0]["model"], "claude-x");
        assert!((reviews[0]["elapsed_seconds"].as_f64().unwrap() - 1.23).abs() < 1e-9);
        assert_eq!(reviews[0]["review"], "looks good"); // trimmed
                                                        // Single-model legacy top-level fields.
        assert_eq!(env["provider"], "anthropic");
        assert_eq!(env["model"], "claude-x");
        assert_eq!(env["review"], "looks good");
        assert!(env.get("elapsed_seconds").is_none()); // top-level omits elapsed
    }

    #[test]
    fn review_envelope_single_model_error_surfaces_as_error() {
        let results = vec![panel_err("openai", None, 0.5, "boom")];
        let env = build_review_envelope("main", None, "", &[], &results);
        assert_eq!(env["file"], serde_json::Value::Null);
        assert_eq!(env["reviews"][0]["error"], "boom");
        assert_eq!(env["reviews"][0]["model"], serde_json::Value::Null);
        assert_eq!(env["error"], "boom"); // legacy top-level error
        assert!(env.get("review").is_none());
    }

    #[test]
    fn review_envelope_multi_model_omits_legacy_top_level() {
        let results = vec![
            panel_ok("anthropic", Some("a"), 1.0, "ok1"),
            panel_err("openai", Some("b"), 2.0, "fail2"),
        ];
        let env = build_review_envelope("dev", None, "", &[], &results);
        assert_eq!(env["reviews"].as_array().unwrap().len(), 2);
        assert_eq!(env["reviews"][0]["review"], "ok1");
        assert_eq!(env["reviews"][1]["error"], "fail2");
        // No top-level legacy fields for a multi-model panel.
        assert!(env.get("provider").is_none());
        assert!(env.get("review").is_none());
        assert!(env.get("error").is_none());
    }

    // ── git read helpers, driven against a real temp repo ──────────────────

    #[test]
    fn default_branch_prefers_origin_head() {
        let repo = crate::test_support::TestRepo::init();
        repo.commit_file("a.txt", "1\n");
        // A symbolic-ref for origin/HEAD wins over the local-branch ladder.
        repo.git(&[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/develop",
        ]);
        assert_eq!(repo.run_in(default_branch).unwrap(), "develop");
    }

    #[test]
    fn default_branch_falls_back_to_main() {
        let repo = crate::test_support::TestRepo::init();
        repo.commit_file("a.txt", "1\n");
        repo.git(&["branch", "-M", "main"]);
        assert_eq!(repo.run_in(default_branch).unwrap(), "main");
    }

    #[test]
    fn default_branch_falls_back_to_master() {
        let repo = crate::test_support::TestRepo::init();
        repo.commit_file("a.txt", "1\n");
        repo.git(&["branch", "-M", "master"]);
        assert_eq!(repo.run_in(default_branch).unwrap(), "master");
    }

    #[test]
    fn default_branch_defaults_to_main_when_none_present() {
        let repo = crate::test_support::TestRepo::init();
        repo.commit_file("a.txt", "1\n");
        // Neither origin/HEAD, main, nor master exists → the "main" fallback.
        repo.git(&["branch", "-M", "feature"]);
        assert_eq!(repo.run_in(default_branch).unwrap(), "main");
    }

    #[test]
    fn get_diff_and_changed_files_report_the_change() {
        let repo = crate::test_support::TestRepo::init();
        repo.commit_file("x.txt", "one\n");
        repo.commit_file("x.txt", "two\n");
        let (diff, files) = repo.run_in(|| {
            (
                get_diff("HEAD~1", None).unwrap(),
                get_changed_files("HEAD~1", None).unwrap(),
            )
        });
        assert!(
            diff.contains("-one"),
            "diff should show the removed line: {diff}"
        );
        assert!(
            diff.contains("+two"),
            "diff should show the added line: {diff}"
        );
        assert_eq!(files, vec!["x.txt".to_string()]);
    }

    #[test]
    fn get_diff_helpers_reject_dash_base() {
        // The `base.starts_with('-')` guard blocks option-injection; it fires
        // before any git call, so no repo is needed.
        assert!(get_diff("-rf", None).is_err());
        assert!(get_diff_stats("-rf", None).is_err());
        assert!(get_changed_files("-rf", None).is_err());
    }

    // ── run_panel fan-out, driven by stub providers (no network) ───────────

    #[test]
    fn run_panel_preserves_order_and_isolates_a_failing_slot() {
        use crate::llm::ProviderKind;
        use crate::test_support::StubLlmProvider;
        // A failing middle slot must not drop or reorder its neighbours.
        let providers: Vec<Box<dyn llm::LlmProvider>> = vec![
            Box::new(StubLlmProvider::ok(ProviderKind::Ollama, Some("m1"), "AAA")),
            Box::new(StubLlmProvider::err(
                ProviderKind::Anthropic,
                Some("m2"),
                "boom",
            )),
            Box::new(StubLlmProvider::ok(ProviderKind::OpenAi, None, "CCC")),
        ];
        let results = run_panel(providers, "the-diff".to_string()).unwrap();

        assert_eq!(results.len(), 3, "a failing slot must not drop the others");
        // Order matches the input despite parallel execution.
        assert_eq!(results[0].provider_kind, "ollama");
        assert_eq!(results[0].model_name.as_deref(), Some("m1"));
        assert_eq!(results[0].outcome.as_deref().unwrap(), "AAA");
        // Middle slot's failure is captured, not propagated.
        assert_eq!(results[1].provider_kind, "anthropic");
        assert_eq!(results[1].model_name.as_deref(), Some("m2"));
        let err = results[1].outcome.as_ref().unwrap_err().to_string();
        assert!(
            err.contains("boom"),
            "captured error should carry the cause: {err}"
        );
        assert_eq!(results[2].provider_kind, "openai");
        assert_eq!(results[2].model_name, None);
        assert_eq!(results[2].outcome.as_deref().unwrap(), "CCC");
    }

    #[test]
    fn run_panel_results_feed_the_multi_model_envelope() {
        use crate::llm::ProviderKind;
        use crate::test_support::StubLlmProvider;
        let providers: Vec<Box<dyn llm::LlmProvider>> = vec![
            Box::new(StubLlmProvider::ok(
                ProviderKind::Ollama,
                Some("m1"),
                "  spaced review  ",
            )),
            Box::new(StubLlmProvider::err(
                ProviderKind::Anthropic,
                Some("m2"),
                "provider exploded",
            )),
        ];
        let results = run_panel(providers, "diff".to_string()).unwrap();
        let env = build_review_envelope(
            "main",
            None,
            "1 file changed",
            &["specX".to_string()],
            &results,
        );

        assert_eq!(env["base"], "main");
        assert_eq!(env["spec_context"][0], "specX");
        let reviews = env["reviews"].as_array().unwrap();
        assert_eq!(reviews.len(), 2);
        assert_eq!(reviews[0]["provider"], "ollama");
        assert_eq!(reviews[0]["model"], "m1");
        assert_eq!(reviews[0]["review"], "spaced review"); // trimmed by insert_result_fields
        assert!(reviews[0]["elapsed_seconds"].is_number());
        assert_eq!(reviews[1]["provider"], "anthropic");
        assert_eq!(reviews[1]["error"], "provider exploded");
        assert!(reviews[1].get("review").is_none());
        // Multi-model panels carry no legacy top-level fields.
        assert!(env.get("review").is_none());
        assert!(env.get("provider").is_none());
    }

    #[test]
    fn run_panel_single_provider_envelope_keeps_legacy_fields() {
        use crate::llm::ProviderKind;
        use crate::test_support::StubLlmProvider;
        let providers: Vec<Box<dyn llm::LlmProvider>> = vec![Box::new(StubLlmProvider::ok(
            ProviderKind::Ollama,
            Some("solo"),
            "the review",
        ))];
        let results = run_panel(providers, "diff".to_string()).unwrap();
        let env = build_review_envelope("main", None, "", &[], &results);

        // Single-model runs mirror the result into legacy top-level fields.
        assert_eq!(env["provider"], "ollama");
        assert_eq!(env["model"], "solo");
        assert_eq!(env["review"], "the review");
        // …and still populate the array form.
        assert_eq!(env["reviews"][0]["review"], "the review");
    }
}
