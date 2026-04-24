---
module: review
version: 6
status: active
files:
  - src/review.rs

db_tables: []
depends_on:
  - spec
  - llm
  - config
---

# Review

## Purpose

AI-powered code review of current branch changes. Gets the git diff against a base branch and sends it to the Claude CLI for review, displaying actionable feedback inline. When the repo is spec-tracked, `review` automatically detects which modules the diff touches (via each spec's `files:` frontmatter and any edits under `specs/<name>/`) and includes their full spec + companion files as *context* for the review. The review target is always the diff itself — specs are reference material.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point for the review command |
| `ReviewOptions` | Options struct with base, file, json, model, prompt, format, with_specs, no_auto_specs |
| `ReviewFormat` | Enum for review output format: Summary, Checklist, or Inline |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ReviewOptions` | `{ base, file, json, model, prompt, format, with_specs, no_auto_specs, provider }` |
| `ReviewFormat` | Enum: `Summary` (default, concise markdown), `Checklist` (markdown checklist), `Inline` (file:line comments) |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(ReviewOptions) -> Result<()>` | Runs AI code review on current diff |
| `build_spec_context` | `(&Path, &[String], &[String], bool) -> Result<Option<(Vec<String>, String)>>` | (private) Assemble auto-detected + explicit spec bundles and return `(names, body)` |
| `build_prompt` | `(&str, &ReviewFormat, Option<&str>, Option<&str>) -> String` | (private) Compose the final prompt: review instructions + optional spec context + diff |
| `get_changed_files` | `(&str, Option<&str>) -> Result<Vec<String>>` | (private) `git diff --name-only` for spec auto-detection |

## Invariants

1. Requires Claude CLI (`claude`) to be installed and authenticated
2. Base branch defaults to auto-detected default: tries `git symbolic-ref refs/remotes/origin/HEAD`, then checks `main` and `master` via `git rev-parse --verify`, falls back to `main`
3. Empty diffs bail with a clear message
4. Shows diff stats before the AI review output
5. `--file` flag restricts review to a single file's changes
6. `--json` outputs structured JSON review results (including the list of specs included in context)
7. `--model` overrides the Claude model used for review
8. `--prompt` appends a custom focus prompt to the default review instructions
9. `--format` controls output style: `summary` (default), `checklist`, or `inline`
10. When the repo has specs, `review` auto-detects relevant modules by intersecting the diff's changed-file list with each spec's frontmatter `files:` field and with the `<specs_dir>/<name>/` directory prefix (respects the `specs_dir` key from `.specsync/config.toml`, defaulting to `specs/`)
11. `--with-specs <names>` (comma-separated, repeatable) appends named modules to the auto-detected set and dedupes
12. `--no-auto-specs` skips the auto-detection step (but still honors `--with-specs`)
13. The review target is always the diff; the prompt explicitly instructs Claude that specs are context-only and that changes outside the diff must not be suggested
14. A broken or missing `.specsync/` never blocks a review — spec auto-detection silently falls back to empty
15. An explicit `--with-specs <name>` that doesn't resolve bails with a clear error naming the missing module
16. `--file <path>` narrows both the diff AND the auto-detection input — only specs whose `files:` or directory intersects that single path will be auto-included. Use `--with-specs` alongside `--file` if you want additional context
17. `--provider {claude,ollama}` overrides env and config for this invocation; `--model` does the same for model selection. The provider chain is identical to `fledge ask`'s (see `llm` spec)
18. JSON output gains `provider` and `model` fields alongside the existing `spec_context` array

## Behavioral Examples

### review — auto-detected spec context (default)
```
$ fledge review
 src/trust.rs | 12 +++-
 specs/trust/context.md | 4 +++
 2 files changed, 15 insertions(+), 1 deletion(-)

Spec context: trust

● Reviewing changes against main ...

[Claude reviews the diff with specs/trust/*.md loaded as context]
```

### review — opt out of auto-detection
```
$ fledge review --no-auto-specs
```

### review — augment with extra specs
```
$ fledge review --with-specs plugin,config
$ fledge review --with-specs plugin --with-specs config
```

### review — against specific base
```
$ fledge review --base develop
```

### review — single file
```
$ fledge review --file src/github.rs
```

### review — json (now includes spec_context)
```
$ fledge review --json
{
  "base": "main",
  "file": null,
  "diff_stats": "...",
  "spec_context": ["trust"],
  "review": "..."
}
```

### review — with custom model and format
```
$ fledge review --model opus --format checklist
```

### review — with custom prompt
```
$ fledge review --prompt "Focus on security vulnerabilities"
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Claude CLI not installed | `claude --version` fails | Bail with install instructions |
| Not a git repo | Outside a git repository | Bail with message |
| No changes | Empty diff against base | Bail with message |
| Claude CLI error | Non-zero exit from claude | Bail with error |
| Invalid format | Unknown `--format` value | Bail with error listing valid formats |
| `--with-specs <name>` not found | Named module has no spec | Bail with "loading spec bundle for '<name>'" context |

## Dependencies

- Active LLM provider — Claude CLI or an Ollama-speaking endpoint (see `llm` spec)
- Git CLI — diff generation, `--name-only` for changed-file detection
- `spec` module — `specs_for_changed_files` (auto-detect), `load_module_bundle` (full spec+companions)
- `llm` module — provider dispatch and construction
- `config` module — `ai.*` section

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 6 | 2026-04-23 | Provider abstraction: `--provider` flag, JSON gains `provider`/`model` fields, invocation routes through `llm::build_provider` instead of shelling out to `claude` directly. Works with Ollama (local or cloud) end-to-end. |
| 5 | 2026-04-23 | Spec-aware review: auto-detect specs for diffed modules (honors `specs_dir` config), `--with-specs`, `--no-auto-specs`, `spec_context` field in JSON output, prompt constraints to keep review target on the diff only |
| 4 | 2026-04-23 | Add ReviewFormat enum, model/prompt/format fields to ReviewOptions |
| 3 | 2026-04-22 | Document default branch fallback algorithm (symbolic-ref → main → master → fallback main) |
| 2 | 2026-04-21 | Add json field to ReviewOptions |
| 1 | 2026-04-19 | Initial spec |
