---
spec: spec.spec.md
---

## User Stories

- As a developer, I want to run `fledge spec check` to validate my specs against source code
- As a developer, I want `fledge spec init` to set up spec-sync configuration in my project
- As a developer, I want `fledge spec new auth` to scaffold a complete spec module with companion files
- As a developer, I want `--strict` mode to treat warnings as errors in CI

## Acceptance Criteria

### REQ-spec-001

The implementation SHALL meet this contract: `fledge spec check` validates all specs in the configured specs directory

### REQ-spec-002

The implementation SHALL meet this contract: `fledge spec check --strict` treats warnings as errors

### REQ-spec-003

The implementation SHALL meet this contract: `fledge spec init` creates `.specsync/` with config.toml, registry.toml, .gitignore, and version

### REQ-spec-004

The implementation SHALL meet this contract: `fledge spec init` creates `specs/` directory if it doesn't exist

### REQ-spec-005

The implementation SHALL meet this contract: `fledge spec new <name>` creates `specs/<name>/` with spec.md and companion files

### REQ-spec-006

The implementation SHALL meet this contract: Validation checks: frontmatter fields, required sections, source file existence

### REQ-spec-007

The implementation SHALL meet this contract: Exit code 1 on errors (or warnings in strict mode), 0 otherwise

### REQ-spec-008

The implementation SHALL meet this contract: Colored output with checkmarks/crosses for each spec

## Constraints

- Must work without network access (no remote resolution)
- Config format must be compatible with spec-sync v4 config.toml
- Companion files use the same frontmatter format as spec-sync

## Out of Scope

- Bidirectional export validation (AST parsing of source code)
- AI-powered spec generation
- Schema/database validation
- Cross-project registry resolution
- Hash caching for incremental validation

### REQ-spec-030

The `spec` module SHALL provide `fledge spec lint`, a two-layer quality gate that
judges the spec itself rather than only code-vs-spec drift.

Acceptance Criteria
- Layer 1 runs offline on every invocation and reports `frontmatter`, `version_format`,
  `missing_section`, `empty_section`, `placeholder_text`, `missing_file`,
  `no_acceptance_signal` and `no_rejection_signal`.
- Layer 2 is model-graded and runs only under `--ai`; `--no-ai` wins when both are passed.
- `--json` emits the action dialect with `action: "spec_lint"` and `schema_version: 1`.
- Exit code is 0 when clean and 1 on any error, or on any warning under `--strict`.

### REQ-spec-031

Layer 2 SHALL fail closed rather than silently skipping when a provider is requested but
unavailable.

Acceptance Criteria
- Provider availability is checked before the first prompt.
- A keyed provider with no key errors naming the environment variable.
- A keyless Ollama gets one short reachability probe so CI fails fast instead of hanging.
- A per-spec provider or parse failure surfaces as a `model_pass_failed` error finding.

### REQ-spec-032

The placeholder check SHALL match its tokens case-insensitively and on word boundaries,
over prose only.

Acceptance Criteria
- `todo`, `Todo` and `FixMe` are reported as placeholders.
- A token appearing inside a longer word (for example "mastodon") is not reported.
- A token inside a fenced block or inline code span is treated as a citation and ignored,
  in any case.

