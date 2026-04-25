---
spec: templates.spec.md
---

## Key Decisions

- Templates are embedded in the binary via `include_dir!` and extracted to a version-stamped cache dir on first use — this ensures `cargo install` users always have templates available
- Three discovery sources: built-in (embedded), extra local dirs (`config.templates.paths`), and remote repos (`config.templates.repos`)
- `template.toml` manifest is required — directories without it are silently skipped
- `.tera` extension is always rendered and stripped; other files match against `files.render` globs; everything else is copied as-is
- Glob matching uses a custom regex-based implementation (not a glob crate) for lightweight matching
- File paths themselves support Tera variables (e.g., `{{ project_name_pascal }}/mod.rs`)

## Files to Read First

- `src/templates.rs` — discovery, rendering, manifest parsing, glob matching
- `templates/` — built-in starter templates (8 starters as of v0.15.2)
- `specs/templates/templates.spec.md` — formal API and invariants

## Current Status

- 8 built-in starter templates embedded in the binary via `include_dir!`: `go-cli`, `kotlin-kmp`, `kotlin-ktor-api`, `python-cli`, `rust-cli`, `static-site`, `ts-bun`, `ts-node`. Additional community templates discoverable via `fledge templates search` (filters on the `fledge-template` GitHub topic).
- Template discovery from built-in, local, and remote sources all working
- Full rendering pipeline: glob matching, Tera rendering, .tera extension stripping, path variable rendering
- Embedded template extraction with version-stamped caching
- 25+ unit tests covering discovery, rendering, globs, and manifests

## Notes

- Embedded template cache lives at `{cache_dir}/fledge/templates-v{version}` — extraction is skipped if the version dir already exists
- `discover_templates_with_repos` fetches remote repos and merges them with local discovery results
- `files.copy` is parsed but not used for rendering decisions — non-rendered files are always copied
