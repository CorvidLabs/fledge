---
module: utils
version: 1
status: active
files:
  - src/utils.rs

db_tables: []
depends_on: []
---

# Utils

## Purpose

Shared cross-cutting utilities used throughout the fledge CLI. Four concerns
live here: the process-wide **non-interactive flag** (set from `--non-interactive`
or `FLEDGE_NON_INTERACTIVE`, plus the interactivity gates that every prompt site
consults), **case conversions** (kebab/camel/snake/pascal) used by templating and
scaffolding, **input validation** for project names, GitHub organizations, and
conventional-commit scopes (the last is a security boundary before untrusted input
reaches an LLM prompt), and **secret redaction** that scrubs credentials and auth
tokens out of user-facing error strings.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `is_non_interactive` | Returns whether the global non-interactive flag is set |
| `set_non_interactive` | Sets the global non-interactive flag (called from `main`) |
| `init_non_interactive_from_env` | Reads `FLEDGE_NON_INTERACTIVE`; flips the flag on a truthy value |
| `is_truthy_env` | Public wrapper over the truthy-string parser for consistent env-var spellings (`1`/`true`/`yes`/`y`/`on`) |
| `is_interactive` | True when stdin is a TTY and the non-interactive flag is not set |
| `require_interactive` | Bails with a flag-named error when a prompt cannot run; the arg is a `--flag` to suggest |
| `require_interactive_hint` | Like `require_interactive` but splices a custom hint (for positional-arg commands) into the error |
| `to_kebab_case` | Converts a string to kebab-case (`_`→`-`, lowercased) |
| `to_camel_case` | Converts a string to camelCase |
| `to_snake_case` | Converts a string to snake_case (`-`→`_`, lowercased) |
| `to_pascal_case` | Converts a string to PascalCase (splitting on `-`/`_`) |
| `validate_project_name` | Rejects empty names, path separators, `..`, null bytes, and Windows-reserved names |
| `validate_github_org` | Rejects empty org names and names containing slashes |
| `validate_commit_scope` | Validates a conventional-commit scope (non-empty, ≤64 chars, ASCII alnum/`-`/`_`) before LLM interpolation |
| `redact_secrets` | Scrubs Authorization/x-access-token headers, URL credentials, and Bearer tokens from a string |

### Structs & Enums

| Type | Description |
|------|-------------|
| (none) | This module exports no structs or enums |

## Invariants

1. The non-interactive flag is a single process-wide `AtomicBool`; `set_non_interactive` and `is_non_interactive` are the only accessors.
2. `is_truthy_env` accepts exactly `1`, `true`, `yes`, `y`, `on` (case-insensitive, trimmed) and rejects everything else, matching the parser used for `FLEDGE_NON_INTERACTIVE`.
3. `is_interactive` returns true only when both stdin is a TTY and the non-interactive flag is unset; setting the flag forces non-interactive regardless of TTY state.
4. `require_interactive` and `require_interactive_hint` return `Ok(())` only when `is_interactive` is true; otherwise they bail with a message that names the escape hatch (a flag, or the provided hint).
5. Case conversions are pure and total: any input (including empty) yields a `String` with no panics.
6. `validate_project_name` rejects empty strings, `/`, `\`, `..`, null bytes, and Windows-reserved device names (`con`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`, case-insensitive).
7. `validate_commit_scope` caps length at 64 characters and permits only ASCII alphanumerics, `-`, and `_`, blocking whitespace and shell/prompt-injection metacharacters at the boundary.
8. `redact_secrets` never emits a real credential: matched header values are replaced to end-of-line, and clean input passes through byte-identical.
9. `validate_github_org` permits spaces but forbids `/` and `\`.

## Behavioral Examples

### Scenario: Non-interactive flag gates a prompt
```
Given FLEDGE_NON_INTERACTIVE=1 has been read via init_non_interactive_from_env
When a command calls require_interactive("yes")
Then it returns Err with a message naming --yes and the FLEDGE_NON_INTERACTIVE escape hatch
```

### Scenario: Case conversion for scaffolding
```
Given the input string "my-cool-project"
When to_pascal_case is called
Then it returns "MyCoolProject"
And to_snake_case returns "my_cool_project"
```

### Scenario: Secret redaction on a failed clone
```
Given the error string "fatal: clone failed: https://user:token@github.com/owner/repo"
When redact_secrets is called
Then the output is "fatal: clone failed: https://[REDACTED]@github.com/owner/repo"
And the token no longer appears in the string
```

## Error Cases

| Error | Condition |
|-------|-----------|
| "requires interactive input but --non-interactive ... is set" | `require_interactive`/`require_interactive_hint` called while the non-interactive flag is set |
| "requires interactive input but stdin is not a TTY" | `require_interactive`/`require_interactive_hint` called when stdin is not a terminal |
| "Project name cannot be empty" | `validate_project_name` given an empty string |
| "Project name cannot contain path separators or '..'" | `validate_project_name` given `/`, `\`, or `..` |
| "'{name}' is a reserved name on Windows" | `validate_project_name` given a Windows device name |
| "GitHub organization cannot be empty" / "... cannot contain slashes" | `validate_github_org` given an empty or slash-containing org |
| "--scope cannot be empty" / "... 64 characters or fewer" / "... only ASCII letters, digits, hyphens, or underscores" | `validate_commit_scope` given empty, over-length, or disallowed-character input |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `std` | `AtomicBool`/`Ordering`, `LazyLock`, `IsTerminal`, `env::var` |
| `anyhow` | `Result`/`bail!` for validation and interactivity errors |
| `regex_lite` | Compiled regex patterns for secret redaction |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-07-03 | Initial spec |
