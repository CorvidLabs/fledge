---
module: ask
version: 3
status: active
files:
  - src/ask.rs

db_tables: []
depends_on:
  - spec
---

# Ask

## Purpose

Ask questions about your codebase using AI. Sends the question to Claude CLI together with a compact index of every module's spec (so Claude knows what modules exist and can cite them), and optionally the full spec + companions for named modules. Streams the answer back.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point for the ask command |
| `AskOptions` | Options struct with question, json, with_specs, and no_spec_index fields |

### Structs & Enums

| Type | Description |
|------|-------------|
| `AskOptions` | `{ question: String, json: bool, with_specs: Vec<String>, no_spec_index: bool }` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(AskOptions) -> Result<()>` | Builds the spec-augmented prompt and delegates to Claude CLI |
| `build_spec_context` | `(&Path, &[String], bool) -> Result<Option<String>>` | (private) Assemble the spec-context block (index + requested bundles) |
| `expand_with_specs` | `(&[String], &Path) -> Result<Vec<String>>` | (private) Flatten comma-separated names; `"all"` expands to every module |
| `build_prompt` | `(&str, bool, Option<&str>) -> String` | (private) Final prompt = preamble + optional spec context + question |

## Invariants

1. Requires Claude CLI (`claude`) to be installed and authenticated
2. Question is joined from multiple args (no quotes required)
3. Claude CLI runs in the current project directory for context
4. `--json` outputs `{question, answer}` structured response
5. By default, a compact spec index is always prepended to the prompt (one line per module: name, version, status, files, first-paragraph purpose). Skipped only when `--no-spec-index` is passed.
6. `--with-specs <names>` (comma-separated or repeated) loads full spec + existing companion files for each named module. `"all"` expands to every spec in the project and supersedes any other names in the same invocation.
7. When no specs exist in the project and no `--with-specs` flag is passed, the prompt is unchanged from the pre-v3 behavior. When `--with-specs <name>` is passed against a project with no specs (or an unknown name), the command bails with a clear error rather than silently succeeding.
8. Spec loading never silently swallows a user-requested bundle: any `--with-specs <name>` that fails to resolve bails with `loading spec bundle for '<name>'`. The ambient index (when `--no-spec-index` is not set) is best-effort — a malformed frontmatter on an unrelated spec is skipped from the index so one bad spec can't break `ask`.
9. Module names passed to `--with-specs` are validated to prevent path traversal: `/`, `\`, `..`, `.`, and empty strings are rejected before any filesystem access.

## Behavioral Examples

### ask — default (index auto-included)
```
$ fledge ask "how does the work module build branch names?"
● Thinking...

[Claude reads the index, knows there's a `work` module, cites specs/work/work.spec.md in its answer]
```

### ask — with full spec + companions for a module
```
$ fledge ask --with-specs work "why does the work module sanitize branch names this way?"
● Thinking...

[Claude has the full spec, context.md design decisions, and requirements.md in its prompt]
```

### ask — multiple specs, comma or repeated
```
$ fledge ask --with-specs work,trust "how do these modules interact?"
$ fledge ask --with-specs work --with-specs trust "how do these modules interact?"
```

### ask — nuclear option
```
$ fledge ask --with-specs all "which modules touch GitHub?"
```

### ask — skip the index (saves tokens)
```
$ fledge ask --no-spec-index "quick syntax question: how do I declare an async trait?"
```

### ask — json
```
$ fledge ask --json "what is the release workflow?"
{
  "question": "what is the release workflow?",
  "answer": "..."
}
```

### ask — no question
```
$ fledge ask
error: Please provide a question. Usage: fledge ask <question>
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Claude CLI not installed | `claude --version` fails | Bail with install instructions |
| No question provided | Empty args | Bail with usage hint |
| `--with-specs <name>` where `specs/<name>/` does not exist | Unknown module | Bail with the looked-at path |
| Claude CLI error | Non-zero exit | Bail with stderr |

## Dependencies

- Claude CLI — AI inference (external dependency)
- `spec` module — `collect_index`, `render_index_markdown`, `load_module_bundle`, `all_module_names`

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 3 | 2026-04-23 | Default-on spec index in prompt; add `--with-specs` for full spec+companion bundles; add `--no-spec-index` escape hatch. Depends on `spec` module helpers. |
| 2 | 2026-04-21 | Add json field to AskOptions |
| 1 | 2026-04-19 | Initial spec |
