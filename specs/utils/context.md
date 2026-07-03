---
spec: utils.spec.md
---

## Key Decisions

- **Single global `AtomicBool` for non-interactivity.** One process-wide flag (set from `--non-interactive` or `FLEDGE_NON_INTERACTIVE`) keeps every prompt site consistent without threading a config value through every call. `set_non_interactive`/`is_non_interactive` are the only accessors.
- **Shared truthy parser.** `is_truthy_env` exposes the same parser used for `FLEDGE_NON_INTERACTIVE` so other env vars (e.g. `FLEDGE_TRUST_HOOKS`) accept identical spellings across the CLI.
- **Two flavors of interactivity guard.** `require_interactive` names a `--flag` to suggest; `require_interactive_hint` splices a custom hint for commands whose required input is a positional arg rather than a flag.
- **Validation as a security boundary.** `validate_commit_scope` restricts scopes to ASCII alnum/`-`/`_` (≤64 chars) specifically to block prompt-injection and shell metacharacters before untrusted input reaches an LLM. `validate_project_name` blocks path traversal and Windows-reserved device names.
- **Defensive secret redaction.** `redact_secrets` scrubs known credential shapes from user-facing error strings, matching header values to end-of-line so multi-token values are fully removed. Regexes compile once via `LazyLock`.

## Files to Read First

- `src/utils.rs` — the entire module (small, single-file)
- `specs/utils/utils.spec.md` — the spec this documents
- `src/main.rs` — where `set_non_interactive` / `init_non_interactive_from_env` are called during startup

## Current Status

Active and stable. All four concerns (non-interactive flag, case conversions, input validation, secret redaction) are implemented and covered by unit tests in the same file.

## Notes

- Test fixtures for `redact_secrets` use obviously-fake `FIXTURE_*_PLACEHOLDER` strings to avoid tripping GitHub secret-scanning push protection on the test file itself.
- The non-interactive flag is process-wide, so tests that flip it share `crate::test_support::NonInteractiveGuard`, which serializes on a mutex to prevent races.
- The redaction threat model is documented in `SECURITY.md` and `remote.rs`.
