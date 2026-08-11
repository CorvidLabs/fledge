---
change: CHG-0009-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
artifact: plan
---

# Plan

## Shape

One new submodule, `src/spec/lint.rs`, alongside `commands.rs` / `parse.rs` /
`validation.rs` / `engine.rs`. Two small additions to the existing parser rather
than a second one. A new clap variant and dispatch arm. Nothing else moves.

## Steps

1. **`src/spec/parse.rs`** — extract `split_frontmatter` out of
   `parse_frontmatter` (raw YAML + body, no interpretation) so lint can inspect
   a `version:` the typed `u32` parser would reject; add
   `extract_section_bodies` so lint can ask "and what is *in* the section",
   which `extract_sections` cannot answer.
2. **`src/spec/mod.rs`** — hoist the seven-section default into a shared
   `DEFAULT_REQUIRED_SECTIONS` (previously an inline `vec![]` in
   `commands::structural_results`), add `SPEC_LINT_SCHEMA`, register the `lint`
   submodule, add `SpecAction::Lint`, and dispatch it.
3. **`src/spec/lint.rs`, layer 1** — `lint_structural(content, root, required)
   -> (SpecMeta, Vec<Finding>)`. Pure apart from the `files:` existence probe,
   so every check is unit-testable with a string and a tempdir. Findings carry a
   stable `check` id, a severity, a layer, an optional section, and a message.
   Supporting pure helpers: `strip_html_comments`, `section_has_content`,
   `version_is_valid`, `frontmatter_value`, `frontmatter_block_has_items`.
4. **`src/spec/lint.rs`, layer 2** — `build_quality_prompt` (spec text + the
   five axes + a bounded `check` vocabulary + a strict JSON output contract),
   `parse_quality_response` (outermost `{...}` span; errors rather than treating
   a non-answer as clean), `normalize_check_id`, `ensure_provider_available`
   (fail fast before the first prompt), `grade_spec` (takes `&dyn LlmProvider`
   so it is testable with the existing `StubLlmProvider`), and
   `model_pass_skip_reason` (pure, flag-only).
5. **Reporting** — `build_envelope` (pure) for `--json`; `print_human` for the
   terminal. `run` orchestrates: resolve targets → layer 1 → optional layer 2 →
   report → `bail!` on failure.
6. **`src/cli.rs` / `src/main.rs`** — `SpecSubcommand::Lint` with `[target]`,
   `--ai`, `--no-ai`, `--provider`, `--model`, `--ignore`, `--strict`, `--json`,
   plus the `spec_action_from` arm.
7. **Tests** — unit tests in `src/spec/tests.rs` (one per layer-1 check, every
   pure helper, the skip/error paths of layer 2 via flags and a stub provider,
   the envelope shape, target resolution); CLI tests in `tests/spec.rs` (exit
   codes, `--json` envelope, a scaffolded spec failing, `--ignore`, an unknown
   `--ignore` id).
8. **Specs** — bump `spec` to v14 (new files entry, exports, invariants,
   behavioral examples, error cases, change log) and `main` to v14
   (`SpecSubcommand` variant list).

## Risks and mitigations

- *Layer 2 hanging in CI* → layer 2 is opt-in, and `ensure_provider_available`
  probes a keyless Ollama once with a 3-second timeout before any prompt.
- *A model inventing check ids* → `normalize_check_id` folds anything unknown
  into `quality_other`, keeping `--ignore` and downstream parsers meaningful.
- *False positives from the emptiness heuristic* → table header/separator rows,
  comments, bare bullets, and lone code fences are treated as structure, and all
  33 of fledge's own specs pass unchanged (verified).
- *Concurrent envelope refactor in other files* → only a new
  `envelope::action` call site is added; no existing call site is touched.
