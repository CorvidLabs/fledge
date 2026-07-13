---
spec: validate.spec.md
---

## User Stories

- As a template author, I want to validate my template before publishing so I catch errors early
- As a CI pipeline, I want to validate templates in strict mode so broken templates don't get merged
- As a developer, I want machine-readable validation output so I can integrate it into tooling

## Acceptance Criteria

### REQ-validate-001

The implementation SHALL meet this contract: Single template validation checks manifest, Tera syntax, variable definitions, and render globs

### REQ-validate-002

The implementation SHALL meet this contract: Batch validation validates all templates in a directory independently

### REQ-validate-003

The implementation SHALL meet this contract: Strict mode exits non-zero on warnings

### REQ-validate-004

The implementation SHALL meet this contract: JSON mode outputs structured ValidationReport array

### REQ-validate-005

The implementation SHALL meet this contract: GitHub Actions `${{ }}` expressions are not flagged as Tera variables

## Constraints

- Must not modify any template files during validation
- Must work on all platforms (Linux, macOS, Windows)

## Out of Scope

- Auto-fixing detected issues
- Validating template output after rendering
