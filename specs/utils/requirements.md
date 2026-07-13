---
spec: utils.spec.md
---

## User Stories

- As an agent, I want a single process-wide non-interactive flag so every prompt site behaves consistently when `--non-interactive` or `FLEDGE_NON_INTERACTIVE` is set
- As a command author, I want `require_interactive` to bail with a message that names the escape-hatch flag so users know how to unblock a scripted run
- As a template/scaffolding author, I want kebab/camel/snake/pascal case converters so generated identifiers match the target convention
- As a security-conscious maintainer, I want project names, GitHub orgs, and commit scopes validated before they reach the filesystem or an LLM prompt
- As a user, I want credentials scrubbed out of error messages so tokens never leak into logs or terminal output

## Acceptance Criteria

### REQ-utils-001

The implementation SHALL meet this contract: `set_non_interactive` / `is_non_interactive` are the only accessors to the global flag

### REQ-utils-002

The implementation SHALL meet this contract: `init_non_interactive_from_env` flips the flag when `FLEDGE_NON_INTERACTIVE` is a truthy value (`1`/`true`/`yes`/`y`/`on`, case-insensitive, trimmed)

### REQ-utils-003

The implementation SHALL meet this contract: `is_interactive` returns true only when stdin is a TTY and the non-interactive flag is unset

### REQ-utils-004

The implementation SHALL meet this contract: `require_interactive` / `require_interactive_hint` return `Ok(())` only when `is_interactive`, otherwise bail with a flag- or hint-named error

### REQ-utils-005

The implementation SHALL meet this contract: Case conversions are pure and total — any input (including empty) returns a `String` without panicking

### REQ-utils-006

The implementation SHALL meet this contract: `validate_project_name` rejects empty strings, `/`, `\`, `..`, null bytes, and Windows-reserved device names

### REQ-utils-007

The implementation SHALL meet this contract: `validate_github_org` rejects empty and slash-containing names but permits spaces

### REQ-utils-008

The implementation SHALL meet this contract: `validate_commit_scope` requires non-empty, ≤64 chars, ASCII alphanumerics plus `-`/`_`

### REQ-utils-009

The implementation SHALL meet this contract: `redact_secrets` scrubs Authorization / x-access-token headers, URL credentials, and Bearer tokens; clean input passes through byte-identical

## Constraints

- The non-interactive flag is a single `AtomicBool` shared across the process — tests must serialize on a guard
- Secret redaction uses `regex_lite` patterns compiled once via `LazyLock`
- `validate_commit_scope` is a security boundary: it runs before untrusted input is interpolated into an LLM prompt or commit message

## Out of Scope

- Prompt rendering itself (lives in `prompts.rs`)
- Redaction of secret shapes beyond the four documented patterns (heuristic token detection, entropy scanning)
- Localization of error messages
