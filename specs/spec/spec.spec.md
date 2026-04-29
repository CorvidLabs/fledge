---
module: spec
version: 8
status: active
files:
  - src/spec/mod.rs
  - src/spec/parse.rs
  - src/spec/validation.rs
  - src/spec/commands.rs
  - src/spec/tests.rs

db_tables: []
depends_on: []
---

# Spec

## Purpose

Integrates spec-sync validation into fledge as native subcommands. Provides `fledge spec check` to validate specs against source code, `fledge spec init` to scaffold a `.specsync/` configuration directory, `fledge spec new <name>` to create a new spec module with companion files, `fledge spec list` to enumerate all specs, and `fledge spec show <name>` to inspect a single spec's structure. Also exposes public helpers (`collect_index`, `render_index_markdown`, `load_module_bundle`, `all_module_names`) for other modules (notably `ask`) to feed spec content into LLM prompts.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate spec subcommand |
| `SpecAction` | Enum of subcommands: Check (strict, json), Init, New, List, Show |
| `SpecFrontmatter` | Parsed YAML frontmatter from a spec file |
| `IndexEntry` | Compact prompt-friendly record of one spec (name, version, status, purpose, files, path) |
| `collect_index` | Enumerate every spec as `IndexEntry`s, sorted by name |
| `render_index_markdown` | Render a slice of `IndexEntry` as a markdown block suitable for prompt injection |
| `load_module_bundle` | Concatenate a module's `.spec.md` and existing companion files into one markdown blob |
| `all_module_names` | Sorted list of every module name with a `.spec.md` file |
| `specs_for_changed_files` | Module names whose `files:` or whose spec file's parent directory intersects a given set of paths |

### Structs & Enums

| Type | Description |
|------|-------------|
| `SpecAction` | Enum of subcommands: Check (strict, json), Init, New, List, Show |
| `SpecFrontmatter` | Parsed YAML frontmatter from a spec file |
| `SpecResult` | Result of validating a single spec (warnings + errors) |
| `SpecSummary` | (private) Summary for `list`: name, version, status, path, files, section/required counts, companions, missing companions |
| `SpecDetail` | (private) Detail for `show`: name, version, status, path, files, sections, companions, missing companions |
| `IndexEntry` | `{name, version, status, purpose: Option<String>, files, path: PathBuf}` |
| `ValidationIssue` | Individual issue: message and is_error flag |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(SpecAction) -> Result<()>` | Dispatches to check, init, new, list, or show |
| `check` | `(root: &Path, strict: bool, json: bool) -> Result<()>` | Validates all specs and prints a human or JSON report (private) |
| `init` | `(root: &Path) -> Result<()>` | Scaffolds `.specsync/` with config.toml, registry.toml, .gitignore, version (private) |
| `new_spec` | `(root: &Path, name: &str) -> Result<()>` | Creates spec directory with spec.md and companion files (private) |
| `list_specs` | `(root: &Path, json: bool) -> Result<()>` | Enumerate specs with frontmatter, section counts, and companion status (private) |
| `show_spec` | `(root: &Path, name: &str, json: bool) -> Result<()>` | Show a single spec's frontmatter, sections, and companion status (private) |
| `collect_index` | `(&Path) -> Result<Vec<IndexEntry>>` | Read every `.spec.md`, parse frontmatter, extract first paragraph of `## Purpose` |
| `render_index_markdown` | `(&[IndexEntry]) -> String` | Format entries as `## Available specs\n- **name** vN (status) — src/foo.rs — purpose` |
| `load_module_bundle` | `(&Path, &str) -> Result<String>` | Spec body + each existing companion, each under its own `### \`filename\`` header |
| `all_module_names` | `(&Path) -> Result<Vec<String>>` | Convenience wrapper over `collect_index` returning just names |
| `specs_for_changed_files` | `(&Path, &[String]) -> Result<Vec<String>>` | Used by `fledge review` to auto-detect which module specs are relevant to a diff |

## Invariants

1. `spec check` exits non-zero if any errors are found (or warnings in strict mode)
2. `spec init` refuses to overwrite an existing `.specsync/` directory
3. `spec new` refuses to overwrite an existing spec directory
4. Frontmatter must contain `module`, `version`, `status`, and `files` fields
5. All files listed in frontmatter `files` must exist on disk
6. All required sections from config must be present in the spec body
7. Companion files (requirements.md, tasks.md, context.md, testing.md) are validated if present
8. `spec list` returns sorted results by module name; `--json` emits `{schema_version: 1, action: "spec_list", specs: [...]}` (with `specs: []` when no specs are present)
9. `spec show` errors if the spec is not found and suggests `fledge spec list`
10. `spec list` and `spec show` are read-only — they never mutate the filesystem
11. `collect_index` silently skips specs whose frontmatter is malformed or files are unreadable, so a single broken spec never breaks a caller like `fledge ask`
12. `collect_index` returns an empty `Vec` (not an error) when the project has no `.specsync/` or no `specs/` directory
13. `load_module_bundle` errors only when the specific requested module is missing; missing companions are simply omitted
14. `render_index_markdown` produces stable output (entries must be pre-sorted; `collect_index` already guarantees this)
15. `specs_for_changed_files` and `load_module_bundle` resolve each spec via its actual on-disk path, so sub-specs that share a directory (e.g. `specs/plugin/plugin-protocol.spec.md` declaring `module: plugin-protocol`) are matched by the parent dir they actually live in. When two specs share a directory, a change under that directory matches both

## Behavioral Examples

### spec check — all valid
```
$ fledge spec check
✓ init (v4, active) — 1 file, 7/7 sections
✓ config (v4, active) — 1 file, 7/7 sections
  2 specs checked, 0 errors, 0 warnings
```

### spec check — missing section
```
$ fledge spec check
✗ init (v4, active) — missing sections: Error Cases
  1 spec checked, 1 error, 0 warnings
```

### spec check — missing source file
```
$ fledge spec check
✗ config (v3, active) — file not found: src/old_config.rs
  1 spec checked, 1 error, 0 warnings
```

### spec check — strict mode with warnings
```
$ fledge spec check --strict
⚠ init (v4, active) — companion file missing: design.md
  1 spec checked, 0 errors, 1 warning (treated as error in strict mode)
```

### spec init — fresh project
```
$ fledge spec init
✓ Created .specsync/config.toml
✓ Created .specsync/registry.toml
✓ Created .specsync/.gitignore
✓ Created .specsync/version
✓ Created specs/
  Spec-sync initialized. Run `fledge spec new <name>` to create your first spec.
```

### spec new — scaffold a module spec
```
$ fledge spec new auth
✓ Created specs/auth/auth.spec.md
✓ Created specs/auth/requirements.md
✓ Created specs/auth/tasks.md
✓ Created specs/auth/context.md
✓ Created specs/auth/testing.md
  Spec module 'auth' created. Edit specs/auth/auth.spec.md to get started.
```

### spec list — enumerate specs
```
$ fledge spec list
● ask v2 (active)
    specs/ask/ask.spec.md — 1 source file, 7/7 sections, 4 companion files
● trust v1 (active)
    specs/trust/trust.spec.md — 1 source file, 7/7 sections, 4 companion files

  32 spec(s) found
```

### spec list --json — machine-readable summary
```
$ fledge spec list --json
{
  "schema_version": 1,
  "action": "spec_list",
  "specs": [
    {
      "name": "trust",
      "version": 1,
      "status": "active",
      "path": "specs/trust/trust.spec.md",
      "files": ["src/trust.rs"],
      "section_count": 7,
      "required_sections": 7,
      "companions": ["requirements.md", "tasks.md", "context.md", "testing.md"],
      "missing_companions": []
    }
  ]
}
```

### spec show — inspect one spec
```
$ fledge spec show trust
trust v1 (active)
  path: specs/trust/trust.spec.md
  source files:
    - src/trust.rs
  sections (7):
    - Purpose
    - Public API
    - Invariants
    - Behavioral Examples
    - Error Cases
    - Dependencies
    - Change Log
  companions:
    ✓ requirements.md
    ✓ tasks.md
    ✓ context.md
    ✓ testing.md
```

### spec show --json — full detail
```
$ fledge spec show trust --json
{
  "schema_version": 1,
  "action": "spec_show",
  "spec": {
    "name": "trust",
    "version": 1,
    "status": "active",
    "path": "specs/trust/trust.spec.md",
    "files": ["src/trust.rs"],
    "sections": ["Purpose", "Public API", "Invariants", ...],
    "companions": ["requirements.md", "tasks.md", "context.md", "testing.md"],
    "missing_companions": []
  }
}
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `.specsync/config.toml` not found | `spec check`, `spec list`, or `spec show` without init | Print helpful message suggesting `fledge spec init` |
| `.specsync/` already exists | `spec init` on initialized project | Bail with message |
| Spec directory already exists | `spec new <name>` where `specs/<name>/` exists | Bail with message |
| Invalid YAML frontmatter | Spec file has malformed frontmatter | `check` reports as error; `list` surfaces as a parse error line; `show` bails with context |
| No specs found | `spec check` or `spec list` with empty specs directory | Print message (or `[]` with `--json`), exit 0 |
| Spec not found | `spec show <name>` with unknown module | Bail with suggestion to run `fledge spec list` |

## Dependencies

- `serde` / `serde_json` — frontmatter parsing and JSON output
- `toml` — config reading/writing
- `walkdir` — spec directory traversal
- `console` — styled terminal output

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 8 | 2026-04-27 | Fix nested-spec resolution (#291). `IndexEntry` now carries the spec file's on-disk `path`. `specs_for_changed_files` matches via each spec's actual parent directory rather than the assumed `<specs_dir>/<name>/`, and `load_module_bundle` resolves the spec file through the index instead of guessing. Sub-specs that share a directory with another module (e.g. `specs/plugin/plugin-protocol.spec.md`) now resolve correctly |
| 7 | 2026-04-26 | Doc sync, behavioral examples for `spec list --json` and `spec show --json` updated to show the post-tier-D envelope shapes (previously displayed the bare-array / bare-detail forms shipped before envelope migration). Invariant 8 reworded to describe the envelope. No code change |
| 6 | 2026-04-26 | Tier-D 1.0 envelope (continuation): all three `--json` paths now wrap output as `{schema_version: 1, action, ...}`. **`spec list --json` is breaking**: bare top-level array → `{schema_version: 1, action: "spec_list", specs: [...]}`. `spec check --json` adds `schema_version`/`action: "spec_check"` (existing fields preserved). `spec show --json` wraps the prior bare detail as `{schema_version: 1, action: "spec_show", spec: {...}}`. Tests updated to assert the envelope shape |
| 5 | 2026-04-23 | Add `--json` to `spec check`. Payload: `{specs: [{name, version, status, file_count, section_count, required_count, errors, warnings}], totals: {checked, errors, warnings}, strict}`. Exit code still non-zero on errors or strict-with-warnings. |
| 4 | 2026-04-23 | Add `specs_for_changed_files` for `review`'s spec auto-detection (matches frontmatter `files:` and `<specs_dir>/<name>/` directory prefix, respecting the configured `specs_dir`) |
| 3 | 2026-04-23 | Expose `collect_index`, `render_index_markdown`, `load_module_bundle`, `all_module_names`, and `IndexEntry` for consumers that need spec content in prompt-friendly form (`ask` is the first such consumer). Add `extract_purpose` helper. |
| 2 | 2026-04-23 | Add `spec list` (alias `ls`) and `spec show`, both with `--json` support for agent/tool consumption |
| 1 | 2026-04-19 | Initial spec for fledge spec integration |
