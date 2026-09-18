---
change: CHG-0009-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
module: spec
---

## ADDED

### REQUIREMENT REQ-spec-030

The `spec` module SHALL provide `fledge spec lint`, a two-layer quality gate that
judges the spec itself rather than only code-vs-spec drift.

Acceptance Criteria
- Layer 1 runs offline on every invocation and reports `frontmatter`, `version_format`,
  `missing_section`, `empty_section`, `placeholder_text`, `missing_file`,
  `no_acceptance_signal` and `no_rejection_signal`.
- Layer 2 is model-graded and runs only under `--ai`; `--no-ai` wins when both are passed.
- `--json` emits the action dialect with `action: "spec_lint"` and `schema_version: 1`.
- Exit code is 0 when clean and 1 on any error, or on any warning under `--strict`.

### REQUIREMENT REQ-spec-031

Layer 2 SHALL fail closed rather than silently skipping when a provider is requested but
unavailable.

Acceptance Criteria
- Provider availability is checked before the first prompt.
- A keyed provider with no key errors naming the environment variable.
- A keyless Ollama gets one short reachability probe so CI fails fast instead of hanging.
- A per-spec provider or parse failure surfaces as a `model_pass_failed` error finding.

### REQUIREMENT REQ-spec-032

The placeholder check SHALL match its tokens case-insensitively and on word boundaries,
over prose only.

Acceptance Criteria
- `todo`, `Todo` and `FixMe` are reported as placeholders.
- A token appearing inside a longer word (for example "mastodon") is not reported.
- A token inside a fenced block or inline code span is treated as a citation and ignored,
  in any case.
