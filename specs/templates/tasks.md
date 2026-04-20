---
spec: templates.spec.md
---

## Tasks

- [x] Define TemplateManifest, TemplateInfo, PromptDef, FileRules, Hooks structs
- [x] Implement template discovery from filesystem directories
- [x] Implement built-in template discovery (embedded via include_dir)
- [x] Implement embedded template extraction with version-stamped caching
- [x] Implement `discover_templates_with_repos` for remote repo integration
- [x] Implement `render_template` with Tera rendering pipeline
- [x] Implement `.tera` extension detection and stripping
- [x] Implement glob-based render/ignore file matching
- [x] Implement path variable rendering (Tera in file/dir names)
- [x] Create built-in starter templates (rust-cli, ts-bun; others migrated to CorvidLabs/fledge-templates)
- [x] Unit tests for discovery, rendering, globs, manifests

## Gaps

- `files.copy` field is parsed but functionally unused (all non-rendered files are copied)
- No template validation command (`fledge check-template`)
- `min_fledge_version` is parsed but not enforced

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
