//! `fledge spec lint` — a quality gate for the spec *itself*.
//!
//! `fledge spec check` answers "does the code match the spec". Nothing answered
//! "is the spec any good" — a thin, aspirational, or poisoned spec passes
//! structurally while being wrong, and the spec is the one input to an agent
//! pipeline that nothing validates. Lint closes that gap in two layers:
//!
//! - **Layer 1, structural** (always runs, deterministic, offline): required
//!   sections present *and non-empty*, no `TODO`/`TBD`/`FIXME` placeholders in
//!   Purpose or Public API, a well-formed `version`, every `files:` entry
//!   present on disk, and at least one acceptance signal plus one rejection
//!   signal.
//! - **Layer 2, model-graded** (opt-in via `--ai`): a model judges the axes a
//!   regex cannot — is the "why" falsifiable, are the invariants real
//!   constraints or decoration, do the acceptance/rejection signals actually
//!   discriminate. Same provider plumbing as `fledge review`.
//!
//! Layer 2 is opt-in rather than opt-out so `spec lint` is safe as a pre-commit
//! hook and a CI gate by default (no network, no spend, no hang). When it *is*
//! requested, provider availability is checked before any prompt is built, so a
//! misconfigured provider fails fast with a clear error instead of hanging.

use anyhow::{bail, Context, Result};
use console::style;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{
    find_spec_files, load_config, parse, specs_dir_from_config, DEFAULT_REQUIRED_SECTIONS,
    SPEC_LINT_SCHEMA,
};
use crate::config::Config;
use crate::llm::{self, ProviderKind, ProviderOverride};

// ── Check identifiers ────────────────────────────────────────────────────────

/// Stable ids for the structural (layer 1) checks. They are part of the CLI
/// contract: `--ignore <id>` and the `check` field of every JSON finding use
/// them, so they must not be renamed without a schema bump.
pub(crate) const STRUCTURAL_CHECKS: &[&str] = &[
    "frontmatter",
    "version_format",
    "missing_section",
    "empty_section",
    "placeholder_text",
    "missing_file",
    "no_acceptance_signal",
    "no_rejection_signal",
];

/// Stable ids the model-graded (layer 2) pass may return. Anything else a model
/// invents is folded into `quality_other`, so `--ignore` stays meaningful and
/// the JSON `check` field has a bounded vocabulary.
pub(crate) const MODEL_CHECKS: &[&str] = &[
    "purpose_not_falsifiable",
    "decorative_invariants",
    "weak_acceptance_signal",
    "weak_rejection_signal",
    "underspecified_error_cases",
    "api_drift",
    "aspirational_claim",
    "quality_other",
];

/// Reported when the layer-2 pass was requested but could not produce a verdict
/// for a spec (provider error, unparseable response). Always an error: a gate
/// that silently did not run is worse than one that fails.
pub(crate) const MODEL_PASS_FAILED: &str = "model_pass_failed";

/// Placeholder tokens that must not appear in Purpose or Public API.
pub(crate) const PLACEHOLDER_TOKENS: &[&str] = &["TODO", "TBD", "FIXME"];

/// Sections whose emptiness is the acceptance / rejection signal, respectively.
pub(crate) const ACCEPTANCE_SECTION: &str = "Behavioral Examples";
pub(crate) const REJECTION_SECTION: &str = "Error Cases";

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Layer {
    Structural,
    Model,
}

/// One lint finding. `check` is a stable id (see [`STRUCTURAL_CHECKS`] /
/// [`MODEL_CHECKS`]), `section` names the spec section it came from when the
/// finding is section-scoped.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Finding {
    pub(crate) check: String,
    pub(crate) severity: Severity,
    pub(crate) layer: Layer,
    pub(crate) section: Option<String>,
    pub(crate) message: String,
}

impl Finding {
    pub(crate) fn error(check: &str, section: Option<&str>, message: impl Into<String>) -> Self {
        Finding {
            check: check.to_string(),
            severity: Severity::Error,
            layer: Layer::Structural,
            section: section.map(str::to_string),
            message: message.into(),
        }
    }
}

/// Frontmatter facts lint needs even when the typed parse fails. `version` is
/// the *raw* string so a non-integer value can be reported rather than swallowed.
#[derive(Debug, Clone, Default)]
pub(crate) struct SpecMeta {
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) status: Option<String>,
}

/// Per-spec lint outcome.
#[derive(Debug)]
pub(crate) struct SpecLintResult {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) meta: SpecMeta,
    pub(crate) findings: Vec<Finding>,
    pub(crate) ignored: usize,
    /// Raw spec file content, retained for the layer-2 prompt.
    pub(crate) content: String,
}

impl SpecLintResult {
    pub(crate) fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    pub(crate) fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }
}

/// Why the layer-2 pass did not run. `None` in [`ModelPass::skipped_reason`]
/// means it did run.
#[derive(Debug, Clone)]
pub(crate) struct ModelPass {
    pub(crate) requested: bool,
    pub(crate) ran: bool,
    pub(crate) skipped_reason: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) model: Option<String>,
}

pub(crate) struct LintOptions {
    /// Module name, spec file path, or directory. `None` lints every spec.
    pub(crate) target: Option<String>,
    pub(crate) json: bool,
    pub(crate) strict: bool,
    /// Run the model-graded layer-2 pass.
    pub(crate) ai: bool,
    /// Never run layer 2 — wins over `ai`.
    pub(crate) no_ai: bool,
    pub(crate) provider: Option<String>,
    pub(crate) model: Option<String>,
    /// Check ids to suppress (the human override). Comma-splittable, repeatable.
    pub(crate) ignore: Vec<String>,
}

// ── Layer 1: structural checks (pure) ────────────────────────────────────────

/// Remove every `<!-- ... -->` block, including multi-line ones. An unterminated
/// comment swallows the remainder — matching how a markdown renderer treats it.
pub(crate) fn strip_html_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        match rest.find("<!--") {
            None => {
                out.push_str(rest);
                return out;
            }
            Some(start) => {
                out.push_str(&rest[..start]);
                match rest[start + 4..].find("-->") {
                    None => return out,
                    Some(end) => rest = &rest[start + 4 + end + 3..],
                }
            }
        }
    }
}

/// Remove fenced code blocks and inline code spans, leaving prose.
///
/// The placeholder check runs on the result: a bare `TODO` in prose is an
/// unfinished spec, but a backticked one is a *citation* — a spec documenting a
/// placeholder-detection feature (this one, for instance) must not fail its own
/// check. HTML comments are deliberately left in: `<!-- TODO: fill in -->` is a
/// placeholder, not a citation.
pub(crate) fn strip_code_spans(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_fence = false;
    for line in input.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        out.push_str(&strip_inline_code(line));
        out.push('\n');
    }
    out
}

/// Drop `` `code` `` spans from one line, matching a run of N backticks with the
/// next run of exactly N. An unmatched run is left alone (it isn't a span).
fn strip_inline_code(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let open_start = i;
        while i < chars.len() && chars[i] == '`' {
            i += 1;
        }
        let run = i - open_start;
        let mut j = i;
        let mut close_end = None;
        while j < chars.len() {
            if chars[j] == '`' {
                let close_start = j;
                while j < chars.len() && chars[j] == '`' {
                    j += 1;
                }
                if j - close_start == run {
                    close_end = Some(j);
                    break;
                }
            } else {
                j += 1;
            }
        }
        match close_end {
            Some(end) => i = end,
            None => {
                out.extend(chars[open_start..].iter());
                return out;
            }
        }
    }
    out
}

/// Strip leading markdown markers (list bullets, ordered-list numbers, heading
/// hashes, blockquote arrows) so `- ` and `1.` don't read as content.
fn strip_markers(input: &str) -> &str {
    let mut s = input;
    loop {
        let start = s;
        s = s.trim_start();
        if let Some(r) = s
            .strip_prefix("- ")
            .or_else(|| s.strip_prefix("* "))
            .or_else(|| s.strip_prefix("+ "))
            .or_else(|| s.strip_prefix("> "))
        {
            s = r;
        } else if s.starts_with('#') {
            s = s.trim_start_matches('#');
        } else {
            let digits = s.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits > 0 {
                let rest = &s[digits..];
                if let Some(r) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
                    s = r;
                }
            }
        }
        if s == start {
            return s;
        }
    }
}

/// A markdown table row: starts and ends with `|`.
fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.ends_with('|') && t.len() > 1
}

/// A markdown table separator row (`|------|:---:|`).
fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    is_table_row(t)
        && t.contains('-')
        && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

/// Does a line carry any information? Scaffold residue — blank lines, empty
/// table cells, bare bullets, lone code fences — does not.
fn line_is_noise(line: &str) -> bool {
    !strip_markers(line).chars().any(|c| c.is_alphanumeric())
}

/// Does a spec section actually say anything?
///
/// Comments are stripped, and a table's header + separator rows are dropped
/// (they are structure, not content) so a scaffolded table of `| | |` reads as
/// empty rather than as "has a Description column".
pub(crate) fn section_has_content(raw: &str) -> bool {
    let cleaned = strip_html_comments(raw);
    let lines: Vec<&str> = cleaned.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if is_table_separator(line) {
            continue;
        }
        // A header row is the row immediately above a separator row.
        if is_table_row(line) && lines.get(i + 1).is_some_and(|n| is_table_separator(n)) {
            continue;
        }
        if !line_is_noise(line) {
            return true;
        }
    }
    false
}

/// Is a `version:` frontmatter value acceptable? fledge/spec-sync specs use a
/// monotonically incremented integer; the issue asks for semver. Both are
/// accepted so the check is meaningful either way — what it rejects is a value
/// that is neither (`v3`, `1.2`, `draft`, empty).
pub(crate) fn version_is_valid(raw: &str) -> bool {
    let v = raw.trim();
    if v.is_empty() {
        return false;
    }
    if v.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    is_semver(v)
}

/// `MAJOR.MINOR.PATCH` with optional `-prerelease` / `+build`. Deliberately
/// small: fledge has no semver dependency and lint only needs a yes/no.
fn is_semver(v: &str) -> bool {
    let without_build = v.split_once('+').map(|(a, _)| a).unwrap_or(v);
    let core = without_build
        .split_once('-')
        .map(|(a, _)| a)
        .unwrap_or(without_build);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Raw value of a top-level `key:` in a frontmatter block, if present.
pub(crate) fn frontmatter_value<'a>(yaml: &'a str, key: &str) -> Option<&'a str> {
    for line in yaml.lines() {
        if line.starts_with([' ', '\t', '-']) {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim() == key {
                return Some(v.trim());
            }
        }
    }
    None
}

/// Is a top-level list/scalar key present *and* non-empty? Used for the
/// optional `accepts:` / `rejects:` frontmatter blocks, which the issue allows
/// as an alternative to non-empty Behavioral Examples / Error Cases.
pub(crate) fn frontmatter_block_has_items(yaml: &str, key: &str) -> bool {
    let mut in_key = false;
    for line in yaml.lines() {
        let is_item = line.starts_with([' ', '\t']) || line.trim_start().starts_with("- ");
        if in_key && is_item && line.trim_start().starts_with("- ") {
            return true;
        }
        if is_item {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim() == key {
                let v = v.trim();
                if !v.is_empty() && v != "[]" {
                    return true;
                }
                in_key = true;
            } else {
                in_key = false;
            }
        }
    }
    false
}

/// Replace a non-integer `version:` value with `0` so the typed frontmatter
/// parser (which requires a `u32`) still yields `module`/`status`/`files` for a
/// spec whose version is a semver string or garbage. The version itself is
/// reported separately by the `version_format` check.
fn normalize_version_for_parse(yaml: &str) -> String {
    yaml.lines()
        .map(|line| {
            if line.starts_with([' ', '\t', '-']) {
                return line.to_string();
            }
            match line.split_once(':') {
                Some((k, v)) if k.trim() == "version" => {
                    if v.trim().chars().all(|c| c.is_ascii_digit()) && !v.trim().is_empty() {
                        line.to_string()
                    } else {
                        format!("{k}: 0")
                    }
                }
                _ => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run every layer-1 check against one spec's raw content.
///
/// Pure apart from the `files:` existence probe, which is why `root` is passed
/// in rather than discovered. Returns the frontmatter facts for reporting plus
/// every finding, in check order.
pub(crate) fn lint_structural(
    content: &str,
    root: &Path,
    required_sections: &[String],
) -> (SpecMeta, Vec<Finding>) {
    let mut meta = SpecMeta::default();
    let mut findings = Vec::new();

    let (yaml, body) = match parse::split_frontmatter(content) {
        Ok(parts) => parts,
        Err(e) => {
            findings.push(Finding::error("frontmatter", None, format!("{e}")));
            return (meta, findings);
        }
    };

    meta.name = frontmatter_value(&yaml, "module").map(str::to_string);
    meta.version = frontmatter_value(&yaml, "version").map(str::to_string);
    meta.status = frontmatter_value(&yaml, "status").map(str::to_string);

    match meta.version.as_deref() {
        None => findings.push(Finding::error(
            "version_format",
            None,
            "frontmatter has no `version` field",
        )),
        Some(v) if !version_is_valid(v) => findings.push(Finding::error(
            "version_format",
            None,
            format!(
                "`version: {v}` is neither an integer nor a semver string (e.g. `4` or `1.2.3`)"
            ),
        )),
        Some(_) => {}
    }

    // Typed parse for `files:`; the version is normalized first so a semver
    // version doesn't cost us the rest of the frontmatter.
    match parse::parse_yaml_frontmatter(&normalize_version_for_parse(&yaml)) {
        Ok(fm) => {
            if meta.name.is_none() {
                meta.name = Some(fm.module.clone());
            }
            for file in &fm.files {
                if !root.join(file).exists() {
                    findings.push(Finding::error(
                        "missing_file",
                        None,
                        format!(
                            "`files:` lists `{file}`, which does not exist — the spec points at code that is gone"
                        ),
                    ));
                }
            }
        }
        Err(e) => findings.push(Finding::error(
            "frontmatter",
            None,
            format!("frontmatter is incomplete: {e}"),
        )),
    }

    let sections = parse::extract_section_bodies(&body);
    let find_section = |name: &str| -> Option<&(String, String)> {
        sections.iter().find(|(heading, _)| heading == name)
    };

    for required in required_sections {
        match find_section(required) {
            None => findings.push(Finding::error(
                "missing_section",
                Some(required),
                format!("required section `## {required}` is missing"),
            )),
            Some((_, section_body)) => {
                if !section_has_content(section_body) {
                    findings.push(Finding::error(
                        "empty_section",
                        Some(required),
                        format!(
                            "section `## {required}` has no content — only scaffolding (blank lines, comments, or empty table rows)"
                        ),
                    ));
                }
            }
        }
    }

    for section in ["Purpose", "Public API"] {
        let Some((_, section_body)) = find_section(section) else {
            continue;
        };
        // Prose only: a backticked `TODO` is a citation, not a placeholder.
        let prose = strip_code_spans(section_body);
        for token in PLACEHOLDER_TOKENS {
            if prose.contains(token) {
                findings.push(Finding::error(
                    "placeholder_text",
                    Some(section),
                    format!("section `## {section}` still contains a `{token}` placeholder"),
                ));
            }
        }
    }

    let has_acceptance = find_section(ACCEPTANCE_SECTION)
        .is_some_and(|(_, b)| section_has_content(b))
        || frontmatter_block_has_items(&yaml, "accepts");
    if !has_acceptance {
        findings.push(Finding::error(
            "no_acceptance_signal",
            Some(ACCEPTANCE_SECTION),
            format!(
                "no acceptance signal — the spec never states what proves success (fill `## {ACCEPTANCE_SECTION}` or add an `accepts:` frontmatter block)"
            ),
        ));
    }

    let has_rejection = find_section(REJECTION_SECTION)
        .is_some_and(|(_, b)| section_has_content(b))
        || frontmatter_block_has_items(&yaml, "rejects");
    if !has_rejection {
        findings.push(Finding::error(
            "no_rejection_signal",
            Some(REJECTION_SECTION),
            format!(
                "no rejection signal — the spec never states what proves failure (fill `## {REJECTION_SECTION}` or add a `rejects:` frontmatter block)"
            ),
        ));
    }

    (meta, findings)
}

// ── Layer 2: model-graded quality pass ───────────────────────────────────────

/// Normalize a model-supplied check id onto [`MODEL_CHECKS`]. Anything
/// unrecognized becomes `quality_other` so `--ignore` and downstream parsers see
/// a bounded vocabulary.
pub(crate) fn normalize_check_id(raw: &str) -> String {
    let slug: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if MODEL_CHECKS.contains(&slug.as_str()) {
        slug
    } else {
        "quality_other".to_string()
    }
}

/// The layer-2 prompt. Only the `.spec.md` itself is sent (not the companion
/// files): the axes being graded — falsifiable purpose, load-bearing invariants,
/// real acceptance/rejection signals — all live in the spec, and a bounded
/// prompt keeps the pass affordable across a whole `specs/` tree.
pub(crate) fn build_quality_prompt(spec_name: &str, spec_content: &str) -> String {
    format!(
        "You are grading the QUALITY OF A SPECIFICATION, not the code it describes.\n\
        A spec is the one input to an automated pipeline that nothing else validates. A thin, \
        aspirational, or subtly wrong spec passes every structural check while being wrong.\n\
        \n\
        Judge the spec below (module `{spec_name}`) on these axes:\n\
        1. Falsifiable purpose — does `## Purpose` state something that could be shown to be false, \
        or is it a vague restatement of the module name?\n\
        2. Load-bearing invariants — is each entry in `## Invariants` a real constraint that could be \
        violated, or decoration (a restatement of the API, an aspiration, a truism)?\n\
        3. Acceptance signal — does `## Behavioral Examples` show concretely what proves the module \
        works, with real inputs and outputs?\n\
        4. Rejection signal — does `## Error Cases` state what proves it is broken, including the \
        observable behavior on failure?\n\
        5. Internal consistency — does `## Public API` agree with the rest of the spec, and does the \
        spec avoid claiming behavior that contradicts itself?\n\
        \n\
        Be conservative. Report only findings you can point at specific text for. Do NOT report \
        missing sections, missing files, or formatting issues — a deterministic pre-pass already \
        covers those. Do NOT review or suggest changes to the code.\n\
        \n\
        Respond with ONLY a JSON object, no prose and no code fence:\n\
        {{\"findings\": [{{\"check\": \"<id>\", \"severity\": \"error\"|\"warning\", \"section\": \"<section name or null>\", \"message\": \"<one sentence, specific>\"}}]}}\n\
        \n\
        `check` must be one of: {checks}.\n\
        Use \"error\" only when the spec would actively mislead an implementer; otherwise \"warning\". \
        An empty findings array is the correct answer for a good spec.\n\
        \n\
        ```markdown\n{spec_content}\n```",
        checks = MODEL_CHECKS.join(", "),
    )
}

#[derive(Debug, serde::Deserialize)]
struct RawModelFinding {
    #[serde(default)]
    check: Option<String>,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    section: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct RawModelResponse {
    #[serde(default)]
    findings: Vec<RawModelFinding>,
}

/// Parse a model response into findings. Tolerates surrounding prose and code
/// fences by taking the outermost `{...}` span; anything else is an error so a
/// non-answer is never mistaken for a clean bill of health.
pub(crate) fn parse_quality_response(raw: &str) -> Result<Vec<Finding>> {
    let start = raw.find('{');
    let end = raw.rfind('}');
    let json = match (start, end) {
        (Some(s), Some(e)) if e > s => &raw[s..=e],
        _ => bail!("model response contained no JSON object"),
    };
    let parsed: RawModelResponse =
        serde_json::from_str(json).context("model response was not the expected JSON shape")?;

    Ok(parsed
        .findings
        .into_iter()
        .filter_map(|f| {
            let message = f
                .message
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty())?;
            let severity = match f.severity.as_deref().map(str::trim) {
                Some(s) if s.eq_ignore_ascii_case("error") => Severity::Error,
                _ => Severity::Warning,
            };
            Some(Finding {
                check: normalize_check_id(f.check.as_deref().unwrap_or("quality_other")),
                severity,
                layer: Layer::Model,
                section: f.section.filter(|s| !s.trim().is_empty()),
                message,
            })
        })
        .collect())
}

/// Fail fast when the provider selected for layer 2 cannot serve a request.
///
/// The default provider is a keyless local Ollama, which in CI is usually not
/// listening — without this the pass would block on a connect timeout per spec.
/// A keyed provider is trusted without a probe; a keyless Ollama is probed once
/// with a short timeout.
pub(crate) fn ensure_provider_available(
    config: &Config,
    provider_override: Option<&str>,
) -> Result<()> {
    let kind = llm::resolve_provider_kind(config, provider_override)?;
    if llm::provider_has_key(kind, config) {
        return Ok(());
    }
    if kind != ProviderKind::Ollama {
        bail!(
            "the model-graded pass needs a provider, but '{}' has no API key.\n  \
             Set {} (or run `fledge ai use <provider> <model>`), or drop --ai to run the structural checks only.",
            kind.as_str(),
            kind.env_var(),
        );
    }
    let host = llm::resolve_effective_host(config, &config.ai.ollama.model, &None);
    if probe_ollama_host(&host) {
        return Ok(());
    }
    bail!(
        "the model-graded pass needs a provider, but the Ollama endpoint {host} is not responding.\n  \
         Start it (`ollama serve`), point fledge at another provider (`fledge ai use <provider> <model>`), \
         or drop --ai to run the structural checks only."
    )
}

/// One short-timeout `GET /api/tags`. Deliberately local to lint rather than
/// shared with `doctor`: the point here is a fast go/no-go before a long
/// generation, not a diagnostic.
fn probe_ollama_host(host: &str) -> bool {
    let url = format!("{}/api/tags", host.trim_end_matches('/'));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into();
    ureq::Agent::get(&agent, &url)
        .header("User-Agent", "fledge-cli")
        .call()
        .is_ok()
}

// ── Target resolution ────────────────────────────────────────────────────────

/// Resolve the `[target]` argument to a sorted list of `.spec.md` paths.
///
/// Accepts a module name (`spec`), a spec file path, or a directory. `None`
/// lints every spec in the configured specs directory.
pub(crate) fn resolve_targets(root: &Path, target: Option<&str>) -> Result<Vec<PathBuf>> {
    let Some(target) = target else {
        let specs_dir = specs_dir_from_config(root)?;
        if !specs_dir.exists() {
            return Ok(Vec::new());
        }
        let mut paths = find_spec_files(&specs_dir);
        paths.sort();
        return Ok(paths);
    };

    let candidate = root.join(target);
    if candidate.is_file() {
        return Ok(vec![candidate]);
    }
    if candidate.is_dir() {
        let mut paths = find_spec_files(&candidate);
        paths.sort();
        if paths.is_empty() {
            bail!("No `.spec.md` files found under {}", candidate.display());
        }
        return Ok(paths);
    }

    // Module name: resolve through the index so nested specs and specs sharing
    // a directory resolve to their real on-disk path.
    if let Some(entry) = super::collect_index(root)?
        .into_iter()
        .find(|e| e.name == target)
    {
        return Ok(vec![entry.path]);
    }

    bail!(
        "No spec found for '{}'. Pass a module name, a `.spec.md` path, or a directory. Run {} to see available specs.",
        target,
        style("fledge spec list").cyan()
    )
}

fn required_sections(root: &Path) -> Vec<String> {
    match load_config(root) {
        Ok(config) if !config.required_sections.is_empty() => config.required_sections,
        _ => DEFAULT_REQUIRED_SECTIONS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

/// Is `id` a check this build can emit? `--ignore` is validated against this so
/// a typo'd id fails loudly instead of silently suppressing nothing.
pub(crate) fn is_known_check(id: &str) -> bool {
    STRUCTURAL_CHECKS.contains(&id) || MODEL_CHECKS.contains(&id) || id == MODEL_PASS_FAILED
}

/// Split and normalize `--ignore` values (repeatable and comma-separated),
/// rejecting ids no check ever emits.
pub(crate) fn parse_ignore_list(raw: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = raw
        .iter()
        .flat_map(|entry| entry.split(','))
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    out.sort();
    out.dedup();
    if let Some(unknown) = out.iter().find(|id| !is_known_check(id)) {
        bail!(
            "unknown --ignore check '{unknown}'. Known checks: {}",
            STRUCTURAL_CHECKS
                .iter()
                .chain(MODEL_CHECKS.iter())
                .copied()
                .chain(std::iter::once(MODEL_PASS_FAILED))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(out)
}

/// Drop findings whose check id the user chose to ignore, returning the number
/// dropped. This is the human override over an agent's judgment.
pub(crate) fn apply_ignores(findings: &mut Vec<Finding>, ignore: &[String]) -> usize {
    if ignore.is_empty() {
        return 0;
    }
    let before = findings.len();
    findings.retain(|f| !ignore.contains(&f.check));
    before - findings.len()
}

// ── Reporting ────────────────────────────────────────────────────────────────

fn spec_json(result: &SpecLintResult, strict: bool) -> serde_json::Value {
    let errors = result.error_count();
    let warnings = result.warning_count();
    serde_json::json!({
        "name": result.name,
        "path": result.path,
        "version": result.meta.version,
        "status": result.meta.status,
        "findings": result.findings,
        "errors": errors,
        "warnings": warnings,
        "ignored": result.ignored,
        "passed": errors == 0 && !(strict && warnings > 0),
    })
}

/// Assemble the `spec_lint` envelope. Pure, so the shape is unit-testable
/// without touching a provider or the filesystem.
pub(crate) fn build_envelope(
    results: &[SpecLintResult],
    model_pass: &ModelPass,
    strict: bool,
) -> serde_json::Value {
    let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
    let total_warnings: usize = results.iter().map(|r| r.warning_count()).sum();
    let total_ignored: usize = results.iter().map(|r| r.ignored).sum();
    let passed = total_errors == 0 && !(strict && total_warnings > 0);
    crate::envelope::action(
        SPEC_LINT_SCHEMA,
        "spec_lint",
        serde_json::json!({
            "strict": strict,
            "model_pass": {
                "requested": model_pass.requested,
                "ran": model_pass.ran,
                "skipped_reason": model_pass.skipped_reason,
                "provider": model_pass.provider,
                "model": model_pass.model,
            },
            "specs": results.iter().map(|r| spec_json(r, strict)).collect::<Vec<_>>(),
            "totals": {
                "linted": results.len(),
                "errors": total_errors,
                "warnings": total_warnings,
                "ignored": total_ignored,
            },
            "passed": passed,
        }),
    )
}

fn print_human(results: &[SpecLintResult], model_pass: &ModelPass, strict: bool) {
    for result in results {
        let errors = result.error_count();
        let warnings = result.warning_count();
        let marker = if errors > 0 || (strict && warnings > 0) {
            style("❌").red().bold()
        } else if warnings > 0 {
            style("⚠️").yellow().bold()
        } else {
            style("✅").green().bold()
        };
        println!(
            "{} {} {}",
            marker,
            style(&result.name).bold(),
            style(format!(
                "(v{}, {})",
                result.meta.version.as_deref().unwrap_or("?"),
                result.meta.status.as_deref().unwrap_or("?")
            ))
            .dim(),
        );
        for finding in &result.findings {
            let styled = match finding.severity {
                Severity::Error => style("error:").red(),
                Severity::Warning => style("warn:").yellow(),
            };
            let scope = match &finding.section {
                Some(s) => format!(" ({s})"),
                None => String::new(),
            };
            println!(
                "    {} [{}]{} {}",
                styled,
                style(&finding.check).cyan(),
                style(scope).dim(),
                finding.message
            );
        }
        if result.ignored > 0 {
            println!(
                "    {} {} finding(s) suppressed by --ignore",
                style("·").dim(),
                result.ignored
            );
        }
    }

    let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
    let total_warnings: usize = results.iter().map(|r| r.warning_count()).sum();
    println!();
    println!(
        "  {} spec(s) linted, {} error(s), {} warning(s)",
        results.len(),
        total_errors,
        total_warnings
    );
    let layer2 = match (&model_pass.ran, &model_pass.skipped_reason) {
        (true, _) => format!(
            "on ({})",
            model_pass
                .provider
                .clone()
                .unwrap_or_else(|| "?".to_string())
        ),
        (false, Some(reason)) => format!("off — {reason}"),
        (false, None) => "off".to_string(),
    };
    println!(
        "  {}",
        style(format!(
            "layer 1 structural: on · layer 2 model-graded: {layer2}"
        ))
        .dim()
    );
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub(crate) fn run(root: &Path, options: LintOptions) -> Result<()> {
    let targets = resolve_targets(root, options.target.as_deref())?;
    let required = required_sections(root);
    let ignore = parse_ignore_list(&options.ignore)?;

    let mut results: Vec<SpecLintResult> = Vec::new();
    for path in &targets {
        let content =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let (meta, mut findings) = lint_structural(&content, root, &required);
        let ignored = apply_ignores(&mut findings, &ignore);
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let name = meta.name.clone().unwrap_or_else(|| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .trim_end_matches(".spec.md")
                .to_string()
        });
        results.push(SpecLintResult {
            name,
            path: rel_path,
            meta,
            findings,
            ignored,
            content,
        });
    }

    let mut model_pass = ModelPass {
        requested: options.ai && !options.no_ai,
        ran: false,
        skipped_reason: model_pass_skip_reason(&options, results.len()),
        provider: None,
        model: None,
    };

    if model_pass.skipped_reason.is_none() {
        run_model_pass(&mut results, &options, &ignore, &mut model_pass)?;
    }

    if results.is_empty() {
        if options.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_envelope(
                    &results,
                    &model_pass,
                    options.strict
                ))?
            );
        } else {
            println!("{} No specs to lint.", style("*").cyan().bold());
        }
        return Ok(());
    }

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_envelope(&results, &model_pass, options.strict))?
        );
    } else {
        print_human(&results, &model_pass, options.strict);
    }

    let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
    let total_warnings: usize = results.iter().map(|r| r.warning_count()).sum();
    if total_errors > 0 || (options.strict && total_warnings > 0) {
        bail!(
            "spec lint failed: {} error(s), {} warning(s)",
            total_errors,
            total_warnings
        );
    }
    Ok(())
}

/// Why layer 2 will not run, or `None` when it will. Pure — the decision is
/// made from the flags alone, before any provider or network work, so the
/// "skipped" path is deterministic and unit-testable.
pub(crate) fn model_pass_skip_reason(options: &LintOptions, spec_count: usize) -> Option<String> {
    if options.no_ai {
        return Some(if options.ai {
            "--no-ai overrides --ai".to_string()
        } else {
            "disabled by --no-ai".to_string()
        });
    }
    if !options.ai {
        return Some("not requested (pass --ai for the model-graded pass)".to_string());
    }
    if spec_count == 0 {
        return Some("no specs to grade".to_string());
    }
    None
}

/// Grade every target spec with the active provider, folding the model's
/// findings into each result.
fn run_model_pass(
    results: &mut [SpecLintResult],
    options: &LintOptions,
    ignore: &[String],
    model_pass: &mut ModelPass,
) -> Result<()> {
    let config = Config::load().context("loading config")?;
    // Fail fast *before* the first prompt: a provider that cannot answer must
    // produce an error, never a per-spec connect timeout.
    ensure_provider_available(&config, options.provider.as_deref())?;

    let provider = llm::build_provider(
        &config,
        &ProviderOverride {
            provider: options.provider.clone(),
            model: options.model.clone(),
        },
    )?;
    model_pass.provider = Some(provider.kind().as_str().to_string());
    model_pass.model = provider.model_name().map(str::to_string);
    model_pass.ran = true;

    let total = results.len();
    for (i, result) in results.iter_mut().enumerate() {
        let sp = crate::spinner::Spinner::start(&format!(
            "Grading spec {}/{} — {} [{}]:",
            i + 1,
            total,
            result.name,
            llm::describe(&*provider)
        ));
        grade_spec(&*provider, result, ignore);
        sp.finish();
    }
    Ok(())
}

/// Grade one spec and fold the model's findings into it. A provider or parse
/// failure becomes a `model_pass_failed` **error** finding on that spec rather
/// than aborting the run: one bad response must not discard the structural
/// verdict for the others, and a gate that silently did not run must not read
/// as a pass.
pub(crate) fn grade_spec(
    provider: &dyn llm::LlmProvider,
    result: &mut SpecLintResult,
    ignore: &[String],
) {
    let prompt = build_quality_prompt(&result.name, &result.content);
    let outcome = provider.invoke(&prompt);
    let mut findings = match outcome.and_then(|raw| parse_quality_response(&raw)) {
        Ok(f) => f,
        Err(e) => vec![Finding {
            check: MODEL_PASS_FAILED.to_string(),
            severity: Severity::Error,
            layer: Layer::Model,
            section: None,
            message: format!("model-graded pass did not produce a verdict: {e:#}"),
        }],
    };
    result.ignored += apply_ignores(&mut findings, ignore);
    result.findings.extend(findings);
}
