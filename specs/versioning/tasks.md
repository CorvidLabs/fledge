# Versioning — Tasks

- [x] Write versioning spec
- [x] Implement versioning.rs with Version struct and comparison
- [x] Integrate check into init.rs (min_fledge_version enforcement)
- [x] Register spec and run full verification

## Gaps

- Version pinning for remote templates is handled in `remote.rs` via `@ref` syntax, not in versioning module
- No `template_version` field in TemplateInfo (template versioning is tracked via git refs instead)
