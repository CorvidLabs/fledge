---
spec: templates.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/templates.rs` (inline) | Unit | Glob matching (exact, `*`, `**`, dot escaping, directory boundaries), path rendering (passthrough, variables, missing vars), manifest parsing (minimal, hooks, prompts, ignore, invalid, missing fields), template discovery (empty paths, nonexistent paths, sorting, extra dirs, dirs without manifest), rendering (.tera files, glob-matched files, copy-only files, ignore rules, path variables, nested dirs, sorted output, missing variable errors), built-in template verification |

## Manual Testing

- [x] `fledge list` shows all 5 built-in templates
- [x] `fledge init my-app --template rust-cli` scaffolds a working Rust CLI project
- [x] Templates from extra config paths appear in `fledge list`
- [x] Remote repo templates appear in `fledge list`
- [x] `.tera` files are rendered and extension is stripped in output
- [x] Files matching `files.render` globs have variables substituted
- [x] Files not matching any render pattern are copied verbatim
- [x] Files matching `files.ignore` globs are excluded from output
- [x] Directory names with `{{ project_name_pascal }}` are rendered correctly
- [x] After `cargo install`, embedded templates are available without the source repo

## Edge Cases & Boundary Conditions

| Scenario | Expected Behavior |
|----------|-------------------|
| No templates found (no builtins, no extra paths) | Returns empty vec (init handles the error) |
| Extra path doesn't exist | Silently skipped |
| Directory without `template.toml` | Silently skipped |
| Invalid TOML in `template.toml` | Error with file path context |
| `template.toml` missing required `description` field | Deserialization error |
| Glob `*.rs` vs `src/main.rs` | No match (`*` doesn't cross directories) |
| Glob `**/*.rs` vs `src/deeply/nested/main.rs` | Matches (recursive) |
| `.tera` file with missing variable | Rendering error with file context |
| Path with `{{ var }}` and missing variable | Rendering error |
| Embedded cache dir already exists for current version | Extraction skipped, cache reused |
| Two templates with same name from different sources | Both included (last in sort wins display, but both exist) |
