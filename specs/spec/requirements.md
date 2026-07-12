---
spec: spec.spec.md
---

## User Stories

- As a developer, I want to run `fledge spec check` to validate my specs against source code
- As a developer, I want `fledge spec init` to set up spec-sync configuration in my project
- As a developer, I want `fledge spec new auth` to scaffold a complete spec module with companion files
- As a developer, I want `--strict` mode to treat warnings as errors in CI

## Durable Requirements

### REQ-spec-001

The implementation SHALL satisfy the following criterion: `fledge spec check` validates all specs in the configured specs directory

Acceptance Criteria

- `fledge spec check` validates all specs in the configured specs directory

### REQ-spec-002

The implementation SHALL satisfy the following criterion: `fledge spec check --strict` treats warnings as errors

Acceptance Criteria

- `fledge spec check --strict` treats warnings as errors

### REQ-spec-003

The implementation SHALL satisfy the following criterion: `fledge spec init` creates `.specsync/` with config.toml, registry.toml, .gitignore, and version

Acceptance Criteria

- `fledge spec init` creates `.specsync/` with config.toml, registry.toml, .gitignore, and version

### REQ-spec-004

The implementation SHALL satisfy the following criterion: `fledge spec init` creates `specs/` directory if it doesn't exist

Acceptance Criteria

- `fledge spec init` creates `specs/` directory if it doesn't exist

### REQ-spec-005

The implementation SHALL satisfy the following criterion: `fledge spec new <name>` creates `specs/<name>/` with spec.md and companion files

Acceptance Criteria

- `fledge spec new <name>` creates `specs/<name>/` with spec.md and companion files

### REQ-spec-006

The implementation SHALL satisfy the following criterion: Validation checks: frontmatter fields, required sections, source file existence

Acceptance Criteria

- Validation checks: frontmatter fields, required sections, source file existence

### REQ-spec-007

The implementation SHALL satisfy the following criterion: Exit code 1 on errors (or warnings in strict mode), 0 otherwise

Acceptance Criteria

- Exit code 1 on errors (or warnings in strict mode), 0 otherwise

### REQ-spec-008

The implementation SHALL satisfy the following criterion: Colored output with checkmarks/crosses for each spec

Acceptance Criteria

- Colored output with checkmarks/crosses for each spec

## Acceptance Criteria

- `fledge spec check` validates all specs in the configured specs directory
- `fledge spec check --strict` treats warnings as errors
- `fledge spec init` creates `.specsync/` with config.toml, registry.toml, .gitignore, and version
- `fledge spec init` creates `specs/` directory if it doesn't exist
- `fledge spec new <name>` creates `specs/<name>/` with spec.md and companion files
- Validation checks: frontmatter fields, required sections, source file existence
- Exit code 1 on errors (or warnings in strict mode), 0 otherwise
- Colored output with checkmarks/crosses for each spec

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
