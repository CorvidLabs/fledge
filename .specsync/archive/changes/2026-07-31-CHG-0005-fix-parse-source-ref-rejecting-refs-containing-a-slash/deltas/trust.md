---
change: CHG-0005-fix-parse-source-ref-rejecting-refs-containing-a-slash
module: trust
---

## MODIFIED

### REQUIREMENT REQ-trust-006

The implementation SHALL meet this contract: `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs, and splits a trailing `@ref` even when the ref itself contains `/` (e.g. a branch name like `chore/0.2.0-launch-prep`)

Acceptance Criteria
- `parse_source_ref("someone/rune@chore/0.2.0-launch-prep")` returns `("someone/rune", Some("chore/0.2.0-launch-prep"))`.
- `parse_source_ref("https://user:token@github.com/owner/repo.git@feature/thing")` returns `("https://user:token@github.com/owner/repo.git", Some("feature/thing"))`.
- `parse_source_ref("https://user:token@github.com/owner/repo.git")` (no trailing ref) still returns `(..., None)` — the credential-URL guard still applies when there is no ref suffix.
