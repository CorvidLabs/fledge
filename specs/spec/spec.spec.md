---
module: spec
version: 15
status: active
files:
  - src/spec/mod.rs
  - src/spec/parse.rs
  - src/spec/validation.rs
  - src/spec/commands.rs
  - src/spec/engine.rs
  - src/spec/lint.rs
  - src/spec/tests.rs

db_tables: []
depends_on: []
---

# Spec

## Purpose

Integrates spec-sync validation into fledge as native subcommands. Provides `fledge spec check` to validate specs against source code, `fledge spec init` to scaffold a `.specsync/` configuration directory, `fledge spec new <name>` to create a new spec module with companion files, `fledge spec list` to enumerate all specs, `fledge spec show <name>` to inspect a single spec's structure, and `fledge spec lint [target]` to gate the quality of the specs themselves. Also exposes public helpers (`collect_index`, `render_index_markdown`, `load_module_bundle`, `all_module_names`) for other modules (notably `ask`) to feed spec content into LLM prompts.

`spec check` asks "does the code match the spec". `spec lint` asks "is the spec worth matching" — a thin, aspirational, or poisoned spec passes every structural check while being wrong, and the spec is the one input to an agent pipeline that nothing else validates. Lint answers that in two layers: a deterministic offline pre-pass (required sections present *and non-empty*, no `TODO`/`TBD`/`FIXME` in Purpose or Public API — case-insensitive, whole-word, prose only — a well-formed `version`, every `files:` entry present on disk, an acceptance signal and a rejection signal), and an opt-in model-graded pass (`--ai`) that judges falsifiability, whether invariants are load-bearing, and whether the acceptance/rejection signals discriminate — using the same provider plumbing as `fledge review`.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point that dispatches to the appropriate spec subcommand |
| `SpecAction` | Enum of subcommands: Check (strict, json), Init, New, List, Show, Lint |
| `SpecFrontmatter` | Parsed YAML frontmatter from a spec file |
| `IndexEntry` | Compact prompt-friendly record of one spec (name, version, status, purpose, files, path) |
| `collect_index` | Enumerate every spec as `IndexEntry`s, sorted by name |
| `render_index_markdown` | Render a slice of `IndexEntry` as a markdown block suitable for prompt injection |
| `load_module_bundle` | Concatenate a module's `.spec.md` and existing companion files into one markdown blob |
| `all_module_names` | Sorted list of every module name with a `.spec.md` file |
| `specs_for_changed_files` | Module names whose `files:` or whose spec file's parent directory intersects a given set of paths |
| `commands` | (internal) Submodule containing spec subcommand implementations |
| `engine` | (internal) Submodule that delegates `spec check` to the real `specsync` binary when installed |
| `lint` | (internal) Submodule implementing `spec lint` — the two-layer spec quality gate |
| `parse` | (internal) Submodule for frontmatter and section parsing |
| `validation` | (internal) Submodule for spec validation logic |
| `find_specsync` | (internal) Locate the `specsync` binary on `PATH`, or `None` if not installed |
| `try_check_via_specsync` | (internal) Run `spec check` via `specsync` when present; `Ok(None)` to fall back to the structural check |
| `COMPANION_FILES` | (internal) List of expected companion filenames: requirements.md, tasks.md, context.md, testing.md |
| `DEFAULT_REQUIRED_SECTIONS` | (internal) The seven-section default set used when `.specsync/config.toml` does not override `required_sections`; shared by `check` and `lint` |
| `SPEC_CHECK_SCHEMA` | (internal) JSON schema version for `spec check --json` output |
| `SPEC_LIST_SCHEMA` | (internal) JSON schema version for `spec list --json` output |
| `SPEC_SHOW_SCHEMA` | (internal) JSON schema version for `spec show --json` output |
| `SPEC_LINT_SCHEMA` | (internal) JSON schema version for `spec lint --json` output |
| `SpecSyncConfig` | (internal) Parsed `.specsync/config.toml` — specs_dir and required_sections |
| `load_config` | (internal) Read and parse `.specsync/config.toml` from project root |
| `find_project_root` | (internal) Return current working directory as the project root |
| `specs_dir_from_config` | (internal) Resolve the specs directory path from config |
| `find_spec_files` | (internal) Walk a directory tree and collect all `.spec.md` file paths |
| `classify_companions` | (internal) Partition companion files into present and missing lists |
| `validate_module_name` | (internal) Reject empty, dot, or path-traversal module names; allow nested names with `/` |
| `module_leaf` | (internal) Last `/`-separated segment of a module name; used to derive the spec filename for nested names |
| `to_title_case` | (internal) Convert snake_case to Title Case for spec scaffolding |
| `parse_frontmatter` | (internal) Parse YAML frontmatter and body from a spec file string |
| `split_frontmatter` | (internal) Split a spec file into its raw YAML block and markdown body without interpreting either |
| `parse_yaml_frontmatter` | (internal) Parse YAML frontmatter fields into `SpecFrontmatter` |
| `extract_sections` | (internal) Extract `## Section` headings from a spec body |
| `extract_section_bodies` | (internal) Extract each `## Section` heading paired with its body (up to the next `## `) |
| `extract_purpose` | (internal) Extract the first paragraph under `## Purpose` |
| `ValidationIssue` | (internal) Individual validation issue with message and is_error flag |
| `SpecResult` | (internal) Aggregate result of validating a single spec |
| `has_errors` | (internal) `SpecResult` method — true if any issue is an error |
| `has_warnings` | (internal) `SpecResult` method — true if any issue is a warning |
| `error_count` | (internal) `SpecResult` method — count of error issues |
| `warning_count` | (internal) `SpecResult` method — count of warning issues |
| `validate_spec` | (internal) Validate a single spec file against project root and required sections |
| `SpecSummary` | (internal) Summary struct for `spec list` output |
| `SpecDetail` | (internal) Detail struct for `spec show` output |
| `check` | (internal) Run spec validation and print human or JSON report |
| `build_summary` | (internal) Parse a spec file into a `SpecSummary` for listing |
| `list_specs` | (internal) Enumerate and display all specs with metadata |
| `show_spec` | (internal) Display detailed view of a single spec |
| `init` | (internal) Scaffold `.specsync/` directory with config and registry |
| `new_spec` | (internal) Create a new spec module directory with template files |
| `STRUCTURAL_CHECKS` | (internal, `lint`) Stable ids for the layer-1 checks; part of the `--ignore` and JSON contract |
| `MODEL_CHECKS` | (internal, `lint`) The bounded vocabulary of check ids the layer-2 pass may return |
| `MODEL_PASS_FAILED` | (internal, `lint`) Check id emitted when layer 2 was requested but produced no verdict |
| `PLACEHOLDER_TOKENS` | (internal, `lint`) `TODO` / `TBD` / `FIXME` — rejected in Purpose and Public API, matched case-insensitively on word boundaries |
| `ACCEPTANCE_SECTION` | (internal, `lint`) The section carrying the success signal (`Behavioral Examples`) |
| `REJECTION_SECTION` | (internal, `lint`) The section carrying the failure signal (`Error Cases`) |
| `error` | (internal, `lint`) `Finding::error` — constructs a structural-layer error finding |
| `Severity` | (internal, `lint`) `error` \| `warning` |
| `Layer` | (internal, `lint`) `structural` \| `model` — which layer produced a finding |
| `Finding` | (internal, `lint`) One lint finding: check id, severity, layer, optional section, message |
| `SpecMeta` | (internal, `lint`) Frontmatter facts (name, raw version, status) usable even when the typed parse fails |
| `SpecLintResult` | (internal, `lint`) Per-spec lint outcome: name, path, meta, findings, ignored count, raw content |
| `ModelPass` | (internal, `lint`) Layer-2 status: requested, ran, skipped reason, provider, model |
| `LintOptions` | (internal, `lint`) Flags for `spec lint`: target, json, strict, ai, no_ai, provider, model, ignore |
| `lint_structural` | (internal, `lint`) Run every layer-1 check against one spec's content, returning `(SpecMeta, Vec<Finding>)` |
| `section_has_content` | (internal, `lint`) Does a section say anything once comments and table scaffolding are removed |
| `strip_html_comments` | (internal, `lint`) Remove `<!-- ... -->` blocks, including multi-line and unterminated ones |
| `strip_code_spans` | (internal, `lint`) Remove fenced code blocks and inline code spans, leaving prose, so a backticked placeholder token reads as a citation rather than a placeholder |
| `contains_placeholder_word` | (internal, `lint`) Does a placeholder token occur in prose as a whole word, ignoring ASCII case — the boundary rule that keeps "mastodon" from reading as a `TODO` |
| `version_is_valid` | (internal, `lint`) Accept an integer or a semver `version:` value; reject anything else |
| `frontmatter_value` | (internal, `lint`) Raw value of a top-level frontmatter key |
| `frontmatter_block_has_items` | (internal, `lint`) Is an optional `accepts:` / `rejects:` block present and non-empty |
| `build_quality_prompt` | (internal, `lint`) Build the layer-2 prompt for one spec |
| `parse_quality_response` | (internal, `lint`) Parse a model response into findings, tolerating fences and prose |
| `normalize_check_id` | (internal, `lint`) Map a model-supplied check id onto `MODEL_CHECKS`, else `quality_other` |
| `ensure_provider_available` | (internal, `lint`) Fail fast before the first prompt when the selected provider cannot answer |
| `model_pass_skip_reason` | (internal, `lint`) Why layer 2 will not run, or `None` when it will |
| `grade_spec` | (internal, `lint`) Grade one spec with a provider and fold the findings in |
| `resolve_targets` | (internal, `lint`) Resolve the `[target]` argument to a sorted list of `.spec.md` paths |
| `is_known_check` | (internal, `lint`) Is an id a check this build can emit |
| `parse_ignore_list` | (internal, `lint`) Split, normalize, and validate `--ignore` values |
| `apply_ignores` | (internal, `lint`) Drop ignored findings, returning how many were dropped |
| `build_envelope` | (internal, `lint`) Assemble the `spec_lint` JSON envelope |

### Structs & Enums

| Type | Description |
|------|-------------|
| `SpecAction` | Enum of subcommands: Check (strict, json), Init, New, List, Show, Lint |
| `SpecFrontmatter` | Parsed YAML frontmatter from a spec file |
| `SpecResult` | Result of validating a single spec (warnings + errors) |
| `Finding` | (private, `lint`) `{check, severity, layer, section: Option<String>, message}` |
| `Severity` | (private, `lint`) `Error` \| `Warning`, serialized lowercase |
| `Layer` | (private, `lint`) `Structural` \| `Model`, serialized lowercase |
| `SpecMeta` | (private, `lint`) `{name, version, status}`, each `Option<String>` — the raw frontmatter facts |
| `SpecLintResult` | (private, `lint`) `{name, path, meta, findings, ignored, content}` |
| `ModelPass` | (private, `lint`) `{requested, ran, skipped_reason, provider, model}` |
| `LintOptions` | (private, `lint`) `{target, json, strict, ai, no_ai, provider, model, ignore}` |
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
| `run` | `(SpecAction) -> Result<()>` | Dispatches to check, init, new, list, show, or lint |
| `lint::run` | `(root: &Path, LintOptions) -> Result<()>` | Runs layer 1 (always) and layer 2 (opt-in), prints a human or JSON report, exits non-zero on failure (private) |
| `lint_structural` | `(content: &str, root: &Path, required: &[String]) -> (SpecMeta, Vec<Finding>)` | Every layer-1 check; pure apart from the `files:` existence probe |
| `section_has_content` | `(&str) -> bool` | False for a section that is only blank lines, comments, bare bullets, or an empty table |
| `strip_code_spans` | `(&str) -> String` | Prose with fenced blocks and inline code spans removed; HTML comments deliberately survive |
| `version_is_valid` | `(&str) -> bool` | True for an integer or a `MAJOR.MINOR.PATCH[-pre][+build]` string |
| `build_quality_prompt` | `(spec_name: &str, spec_content: &str) -> String` | Layer-2 prompt; sends the `.spec.md` only, not the companions |
| `parse_quality_response` | `(&str) -> Result<Vec<Finding>>` | Outermost `{...}` span → findings; errors rather than treating a non-answer as clean |
| `ensure_provider_available` | `(&Config, Option<&str>) -> Result<()>` | Keyed providers trusted without a probe; a keyless Ollama gets one 3s `GET /api/tags` |
| `model_pass_skip_reason` | `(&LintOptions, spec_count: usize) -> Option<String>` | Pure flag-only decision, made before any provider work |
| `grade_spec` | `(&dyn LlmProvider, &mut SpecLintResult, &[String])` | Grades one spec; a failure becomes a `model_pass_failed` error finding |
| `resolve_targets` | `(&Path, Option<&str>) -> Result<Vec<PathBuf>>` | Module name, `.spec.md` path, directory, or all specs |
| `build_envelope` | `(&[SpecLintResult], &ModelPass, strict: bool) -> serde_json::Value` | The `spec_lint` envelope (private, pure) |
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
16. `spec lint` layer 1 always runs and never performs network I/O — a bare `fledge spec lint` is safe as a pre-commit hook and a CI gate
17. `spec lint` layer 2 is opt-in: it runs only with `--ai`, and `--no-ai` always wins over `--ai`. When it is skipped, `model_pass.ran` is `false` and `model_pass.skipped_reason` names why
18. When layer 2 is requested but the selected provider cannot answer, `spec lint` errors *before* building the first prompt (keyed providers are trusted; a keyless Ollama is probed once with a 3-second timeout). It never blocks on a per-spec connect timeout
19. A layer-2 provider or parse failure for one spec becomes a `model_pass_failed` **error** finding on that spec, not a silent pass and not an aborted run — a gate that could not run must never read as clean
20. `spec lint` exits non-zero when any finding is an error, or when `--strict` and any finding is a warning; exit 0 when clean. Errors go to stderr as plain text even under `--json`
21. Every `check` id in the output comes from a closed vocabulary (`STRUCTURAL_CHECKS` ∪ `MODEL_CHECKS` ∪ `model_pass_failed`). Ids a model invents collapse to `quality_other`, and `--ignore` rejects any id outside that set rather than silently suppressing nothing
22. `--ignore <check>` is the human override over the agent's judgment: matching findings are dropped from the verdict and counted in `ignored`
23. `version:` is accepted as either a spec-sync integer or a semver string. A non-integer version does not cost the spec its other frontmatter checks — lint normalizes the value before the typed parse
24. The placeholder check runs on **prose only** — fenced code blocks and inline code spans are stripped first, so a spec that documents placeholder detection (this one) does not fail its own check. HTML comments are *not* stripped: a commented-out placeholder is still a placeholder
25. Within that prose, a placeholder token matches **case-insensitively and on word boundaries**: a lowercase `todo` or a mixed-case `FixMe` is as unfinished as `TODO`, while a token buried in a longer word ("mastodon", "prefixmethod") is not a finding. A boundary is any position whose adjacent character is neither alphanumeric nor `_`
26. `spec lint` is read-only; it never mutates the filesystem

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

### spec lint — a clean tree
```
$ fledge spec lint
✅ spec (v14, active)
✅ work (v15, active)

  33 spec(s) linted, 0 error(s), 0 warning(s)
  layer 1 structural: on · layer 2 model-graded: off — not requested (pass --ai for the model-graded pass)
$ echo $?
0
```

### spec lint — a freshly scaffolded spec is structurally complete but says nothing
```
$ fledge spec new auth && fledge spec lint auth
❌ auth (v1, draft)
    error: [missing_file] `files:` lists `src/auth.rs`, which does not exist — the spec points at code that is gone
    error: [empty_section] (Public API) section `## Public API` has no content — only scaffolding (blank lines, comments, or empty table rows)
    error: [empty_section] (Invariants) section `## Invariants` has no content — only scaffolding (blank lines, comments, or empty table rows)
    error: [empty_section] (Error Cases) section `## Error Cases` has no content — only scaffolding (blank lines, comments, or empty table rows)
    error: [no_rejection_signal] (Error Cases) no rejection signal — the spec never states what proves failure (fill `## Error Cases` or add a `rejects:` frontmatter block)

  1 spec(s) linted, 5 error(s), 0 warning(s)
error: spec lint failed: 5 error(s), 0 warning(s)
$ echo $?
1
```

### spec lint --json — the CI gate
```
$ fledge spec lint --json
{
  "schema_version": 1,
  "action": "spec_lint",
  "strict": false,
  "model_pass": {
    "requested": false,
    "ran": false,
    "skipped_reason": "not requested (pass --ai for the model-graded pass)",
    "provider": null,
    "model": null
  },
  "specs": [
    {
      "name": "auth",
      "path": "specs/auth/auth.spec.md",
      "version": "1",
      "status": "draft",
      "findings": [
        {
          "check": "no_rejection_signal",
          "severity": "error",
          "layer": "structural",
          "section": "Error Cases",
          "message": "no rejection signal — the spec never states what proves failure ..."
        }
      ],
      "errors": 1,
      "warnings": 0,
      "ignored": 0,
      "passed": false
    }
  ],
  "totals": { "linted": 1, "errors": 1, "warnings": 0, "ignored": 0 },
  "passed": false
}
```

### spec lint --ai — the model-graded pass
```
$ fledge spec lint spec --ai
⚠️ spec (v14, active)
    warn: [decorative_invariants] (Invariants) Invariant 10 restates the API rather than constraining it.
    warn: [weak_acceptance_signal] (Behavioral Examples) The examples show shapes but never a concrete input/output pair.

  1 spec(s) linted, 0 error(s), 2 warning(s)
  layer 1 structural: on · layer 2 model-graded: on (ollama)
```

### spec lint --ai without a reachable provider — fails fast, never hangs
```
$ fledge spec lint --ai --provider anthropic
error: the model-graded pass needs a provider, but 'anthropic' has no API key.
  Set ANTHROPIC_API_KEY (or run `fledge ai use <provider> <model>`), or drop --ai to run the structural checks only.
$ echo $?
1
```

### spec lint --ignore — the human override
```
$ fledge spec lint auth --ignore missing_file,empty_section
❌ auth (v1, draft)
    error: [no_rejection_signal] (Error Cases) no rejection signal — ...
    · 4 finding(s) suppressed by --ignore
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `.specsync/config.toml` not found | `spec check`, `spec list`, `spec show`, or `spec lint` without init | Print helpful message suggesting `fledge spec init` |
| `.specsync/` already exists | `spec init` on initialized project | Bail with message |
| Spec directory already exists | `spec new <name>` where `specs/<name>/` exists | Bail with message |
| Invalid YAML frontmatter | Spec file has malformed frontmatter | `check` reports as error; `list` surfaces as a parse error line; `show` bails with context |
| No specs found | `spec check` or `spec list` with empty specs directory | Print message (or `[]` with `--json`), exit 0 |
| Spec not found | `spec show <name>` with unknown module | Bail with suggestion to run `fledge spec list` |
| Lint target not found | `spec lint <target>` where the target is neither a module name, a `.spec.md` path, nor a directory | Bail naming the three accepted forms and suggesting `fledge spec list` |
| Empty directory target | `spec lint <dir>` containing no `.spec.md` files | Bail with "No `.spec.md` files found under \<dir\>" |
| Unknown `--ignore` id | `spec lint --ignore nope` | Bail listing every known check id (exit 1); no lint runs |
| No provider for layer 2 | `spec lint --ai` with a keyless keyed provider, or an unreachable Ollama endpoint | Bail before the first prompt, naming the env var to set and the `--ai` escape hatch (exit 1) |
| Layer-2 provider or parse failure | `spec lint --ai` where a model errors or returns non-JSON | `model_pass_failed` error finding on that spec; other specs still graded; run exits non-zero |
| Findings present | `spec lint` with any error (or any warning under `--strict`) | Print the report (human or `--json` envelope) on stdout, then a plain-text error on stderr; exit 1 |

## Dependencies

- `serde` / `serde_json` — frontmatter parsing and JSON output
- `toml` — config reading/writing
- `walkdir` — spec directory traversal
- `console` — styled terminal output
- `ureq` — the layer-2 provider reachability probe (`lint`)
- `llm` / `config` (internal) — provider selection for the model-graded pass, shared with `review` and `ask`
- `envelope` (internal) — `--json` envelope construction
- `spinner` (internal) — progress display during the model-graded pass

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 14 | 2026-07-30 | Add `fledge spec lint [target]` (#429) — a quality gate for the spec itself, in two layers. Layer 1 is a deterministic offline pre-pass (required sections present and non-empty, no `TODO`/`TBD`/`FIXME` in Purpose or Public API — matched case-insensitively and on word boundaries, against prose with code spans stripped, via `contains_placeholder_word` — integer-or-semver `version`, every `files:` entry present, an acceptance signal and a rejection signal). Layer 2 is an opt-in model-graded pass (`--ai`) reusing `review`'s provider plumbing, judging falsifiable purpose, load-bearing invariants, and discriminating acceptance/rejection signals. New `src/spec/lint.rs`; new `split_frontmatter` / `extract_section_bodies` in `parse.rs`; new shared `DEFAULT_REQUIRED_SECTIONS`. `--json` emits `{schema_version: 1, action: "spec_lint", strict, model_pass, specs: [...], totals, passed}`. `--ignore <check>` is the human override; `--strict` promotes warnings to errors |
| 13 | 2026-07-02 | Fix: the specsync-delegated `spec check --json` now includes the full `specs[]` structural inventory (`{name, version, status, file_count, section_count, required_count, errors, warnings}`), so the `--json` envelope shape is identical whether the engine is `structural` or `specsync` (previously the delegated path omitted `specs[]`, breaking the documented contract and any agent parsing it on a machine with specsync installed). specsync's aggregate verdict (`passed`, top-level `errors`/`warnings`, `stale`) is preserved. New shared `structural_results`/`spec_result_json` helpers back both engines' `specs[]` |
| 12 | 2026-06-22 | `spec check` now delegates to the real `specsync` binary when it is on `PATH`, giving local runs the same export-coverage validation as CI (identical to `CorvidLabs/spec-sync`); falls back to the built-in structural check (with an install hint) when absent. New `engine` submodule (`src/spec/engine.rs`) holds `find_specsync` and `try_check_via_specsync`. JSON output gains an `engine` field (`"specsync"` or `"structural"`) |
| 11 | 2026-06-03 | Document `parse_yaml_frontmatter` in the export table to satisfy strict spec-sync validation |
| 10 | 2026-05-11 | Accept nested module names with `/` for `fledge spec new` (#383). `validate_module_name` allows `game/board`-style names while still rejecting `\`, leading/trailing `/`, `//`, and any `..`/`.` segment. New `module_leaf` helper derives the spec filename for nested names (`game/board` → `board.spec.md`). `new_spec` writes nested directory layout and quotes registry keys containing `/` so the resulting TOML stays valid |
| 9 | 2026-04-29 | Document all `pub(crate)` exports from module split (`mod.rs`, `parse.rs`, `validation.rs`, `commands.rs`) to satisfy strict spec-sync validation |
| 8 | 2026-04-27 | Fix nested-spec resolution (#291). `IndexEntry` now carries the spec file's on-disk `path`. `specs_for_changed_files` matches via each spec's actual parent directory rather than the assumed `<specs_dir>/<name>/`, and `load_module_bundle` resolves the spec file through the index instead of guessing. Sub-specs that share a directory with another module (e.g. `specs/plugin/plugin-protocol.spec.md`) now resolve correctly |
| 7 | 2026-04-26 | Doc sync, behavioral examples for `spec list --json` and `spec show --json` updated to show the post-tier-D envelope shapes (previously displayed the bare-array / bare-detail forms shipped before envelope migration). Invariant 8 reworded to describe the envelope. No code change |
| 6 | 2026-04-26 | Tier-D 1.0 envelope (continuation): all three `--json` paths now wrap output as `{schema_version: 1, action, ...}`. **`spec list --json` is breaking**: bare top-level array → `{schema_version: 1, action: "spec_list", specs: [...]}`. `spec check --json` adds `schema_version`/`action: "spec_check"` (existing fields preserved). `spec show --json` wraps the prior bare detail as `{schema_version: 1, action: "spec_show", spec: {...}}`. Tests updated to assert the envelope shape |
| 5 | 2026-04-23 | Add `--json` to `spec check`. Payload: `{specs: [{name, version, status, file_count, section_count, required_count, errors, warnings}], totals: {checked, errors, warnings}, strict}`. Exit code still non-zero on errors or strict-with-warnings. |
| 4 | 2026-04-23 | Add `specs_for_changed_files` for `review`'s spec auto-detection (matches frontmatter `files:` and `<specs_dir>/<name>/` directory prefix, respecting the configured `specs_dir`) |
| 3 | 2026-04-23 | Expose `collect_index`, `render_index_markdown`, `load_module_bundle`, `all_module_names`, and `IndexEntry` for consumers that need spec content in prompt-friendly form (`ask` is the first such consumer). Add `extract_purpose` helper. |
| 2 | 2026-04-23 | Add `spec list` (alias `ls`) and `spec show`, both with `--json` support for agent/tool consumption |
| 1 | 2026-04-19 | Initial spec for fledge spec integration |
| 15 | 2026-08-11 | CHG-0009-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429: Fledge spec lint: two-layer quality gate for the spec itself (issue #429) |
