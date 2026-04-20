---
spec: validate.spec.md
---

## User Stories

- As a template author, I want to validate my template before publishing so I catch errors early
- As a CI pipeline, I want to validate templates in strict mode so broken templates don't get merged
- As a developer, I want machine-readable validation output so I can integrate it into tooling

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
