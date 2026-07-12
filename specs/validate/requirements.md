---
spec: validate.spec.md
---

## User Stories

- As a template author, I want to validate my template before publishing so I catch errors early
- As a CI pipeline, I want to validate templates in strict mode so broken templates don't get merged
- As a developer, I want machine-readable validation output so I can integrate it into tooling

## Durable Requirements

### REQ-validate-001

The implementation SHALL satisfy the following criterion: Single template validation checks manifest, Tera syntax, variable definitions, and render globs

Acceptance Criteria

- Single template validation checks manifest, Tera syntax, variable definitions, and render globs

### REQ-validate-002

The implementation SHALL satisfy the following criterion: Batch validation validates all templates in a directory independently

Acceptance Criteria

- Batch validation validates all templates in a directory independently

### REQ-validate-003

The implementation SHALL satisfy the following criterion: Strict mode exits non-zero on warnings

Acceptance Criteria

- Strict mode exits non-zero on warnings

### REQ-validate-004

The implementation SHALL satisfy the following criterion: JSON mode outputs structured ValidationReport array

Acceptance Criteria

- JSON mode outputs structured ValidationReport array

### REQ-validate-005

The implementation SHALL satisfy the following criterion: GitHub Actions `${{ }}` expressions are not flagged as Tera variables

Acceptance Criteria

- GitHub Actions `${{ }}` expressions are not flagged as Tera variables

## Acceptance Criteria

- Single template validation checks manifest, Tera syntax, variable definitions, and render globs
- Batch validation validates all templates in a directory independently
- Strict mode exits non-zero on warnings
- JSON mode outputs structured ValidationReport array
- GitHub Actions `${{ }}` expressions are not flagged as Tera variables

## Constraints

- Must not modify any template files during validation
- Must work on all platforms (Linux, macOS, Windows)

## Out of Scope

- Auto-fixing detected issues
- Validating template output after rendering
