---
spec: create_template.spec.md
---

## Completed

- [x] Implement `CreateTemplateOptions` struct and `run()` entry point
- [x] Interactive prompts with defaults (name, description, render globs, hooks, prompts)
- [x] Scaffold directory with `template.toml`, example files, README
- [x] Generated `template.toml` validated as parseable `TemplateManifest`
- [x] Wire into CLI as `fledge create-template <name> [-o <dir>]`
- [x] Unit tests (5 tests covering scaffold output, manifest validity, error cases)
- [x] Spec files (spec, context, requirements, tasks, testing)

- [x] Add `--yes` flag to skip prompts and accept all defaults
- [x] Add `fledge validate` command for template validation
