---
spec: validate.spec.md
---

## Context

Template validation catches errors before publishing — broken Tera syntax, missing variables, incomplete manifests. Running `fledge validate-template` before `fledge publish` prevents distributing broken templates to the community.

## Related Modules

- `templates` — provides `TemplateManifest` and `matches_glob_pub` for manifest parsing and glob matching

## Design Decisions

- GitHub Actions `${{ }}` expressions are excluded from Tera variable detection — they look similar but are unrelated
- Builtin variables are hardcoded — they're stable and defined by the template engine
- Undefined variables are warnings not errors — templates may rely on user-provided extra context
- Batch mode validates entire template directories — useful for CI on template repos
