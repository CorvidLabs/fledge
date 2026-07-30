---
change: CHG-0007-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
artifact: testing
---

# Testing

## Principle

No test makes a real network call to an LLM. Layer 2 is exercised through a
pure flag-only decision function (`model_pass_skip_reason`), the existing
`StubLlmProvider` double, and pure response parsing.

## Unit tests — `src/spec/tests.rs`

Layer 1, one assertion focus each, all driven off a single `healthy_spec()`
fixture that passes every check, mutated one piece at a time:

| Test | Proves |
|------|--------|
| `test_lint_healthy_spec_has_no_findings` | The fixture is genuinely clean; `SpecMeta` is populated |
| `test_lint_reports_missing_frontmatter` | `frontmatter` fires and short-circuits |
| `test_lint_reports_missing_source_file` | `missing_file` names the vanished path |
| `test_lint_reports_missing_required_section` | `missing_section` is section-scoped |
| `test_lint_reports_empty_section_of_pure_scaffolding` | `empty_section` fires on a comment-only section |
| `test_lint_reports_placeholder_tokens_in_purpose_and_public_api` | `placeholder_text` fires in both guarded sections |
| `test_lint_accepts_integer_and_semver_versions_but_rejects_others` | `version_format` accepts `3`, `1.2.3`, `0.1.0-rc.1`; rejects `v3`, `1.2`, `draft` |
| `test_lint_semver_version_still_yields_the_rest_of_the_frontmatter` | A semver version does not cost the spec its `files:` checks |
| `test_lint_reports_missing_acceptance_and_rejection_signals` | Both signal checks fire, alongside (not instead of) `empty_section` |
| `test_lint_accepts_and_rejects_frontmatter_blocks_supply_the_signals` | `accepts:` / `rejects:` satisfy the signal requirement |
| `test_lint_check_ids_are_a_closed_vocabulary` | Every declared id is emittable; unknown ids are not |

Pure helpers: `test_strip_html_comments_handles_multiline_and_unterminated`,
`test_section_has_content_ignores_scaffolding`, `test_version_is_valid`,
`test_frontmatter_value_reads_top_level_keys_only`,
`test_frontmatter_block_has_items`.

Override: `test_parse_ignore_list_splits_dedupes_and_validates`,
`test_apply_ignores_drops_matching_findings_and_counts_them`.

Layer 2 (no network): `test_build_quality_prompt_grades_the_spec_not_the_code`,
`test_parse_quality_response_accepts_fenced_and_prose_wrapped_json`,
`test_parse_quality_response_defaults_severity_and_normalizes_unknown_checks`,
`test_parse_quality_response_drops_empty_messages_and_rejects_non_json`,
`test_normalize_check_id_maps_onto_the_allowlist`,
`test_model_pass_is_skipped_unless_requested` (default off, `--no-ai` beats
`--ai`, nothing to grade, and the one case where it runs),
`test_ensure_provider_available_errors_when_the_selected_provider_has_no_key`
(env-locked, asserts the message names both the env var and the escape hatch),
`test_grade_spec_folds_model_findings_into_the_result`,
`test_grade_spec_turns_a_provider_failure_into_a_model_pass_failed_error`,
`test_grade_spec_honors_the_ignore_list`.

Reporting and targets: `test_build_envelope_shape_and_pass_verdict`,
`test_build_envelope_fails_on_errors_and_on_strict_warnings`,
`test_resolve_targets_by_module_name_path_and_directory`.

## Integration tests — `tests/spec.rs`

| Test | Proves |
|------|--------|
| `cli_spec_lint_succeeds_in_project` | fledge's own 33 specs clear the gate; exit 0 |
| `cli_spec_lint_json_envelope` | Envelope keys, `model_pass.ran == false` by default (no provider touched in CI) |
| `cli_spec_lint_single_module` | Module-name targeting lints exactly one spec |
| `cli_spec_lint_rejects_a_freshly_scaffolded_spec` | The headline case: `spec new` output fails lint; exit non-zero; error text on stderr while the envelope is on stdout |
| `cli_spec_lint_ignore_suppresses_a_check` | `--ignore` drops findings and increments `ignored` |
| `cli_spec_lint_rejects_unknown_ignore_id` | A typo'd id is an error, not a silent no-op |

## Manual verification

- `fledge spec lint` over the repo: 33 specs, 0 errors, 0 warnings, exit 0.
- `fledge spec lint spec --json | jq .passed` → `true`.

## Gates

`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and
`specsync check` export coverage for the `spec` and `main` modules.
