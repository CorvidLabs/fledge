---
spec: trust.spec.md
---

## User Stories

- As a user installing plugins/templates/lanes, I want to see whether the source is local, official, team, or unverified so I can make informed trust decisions
- As a module author, I want a shared trust classification function so all extension types use consistent logic

## Acceptance Criteria

### REQ-trust-001

The implementation SHALL meet this contract: `determine_trust_tier` classifies `CorvidLabs/*` sources as Official

### REQ-trust-002

The implementation SHALL meet this contract: `determine_trust_tier` classifies filesystem path sources as Local

### REQ-trust-003

The implementation SHALL meet this contract: `determine_trust_tier` classifies sources owned by a human member of the CorvidLabs org (e.g. `0xLeif/*`) as Team

### REQ-trust-004

The implementation SHALL meet this contract: `determine_trust_tier` classifies all other sources as Unverified

### REQ-trust-005

The implementation SHALL meet this contract: Supports local paths, HTTPS URLs, SSH URLs, and `owner/repo` shorthand

### REQ-trust-006

The implementation SHALL meet this contract: `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs, and splits a trailing `@ref` even when the ref itself contains `/` (e.g. a branch name like `chore/0.2.0-launch-prep`)

Acceptance Criteria
- `parse_source_ref("someone/rune@chore/0.2.0-launch-prep")` returns `("someone/rune", Some("chore/0.2.0-launch-prep"))`.
- `parse_source_ref("https://user:token@github.com/owner/repo.git@feature/thing")` returns `("https://user:token@github.com/owner/repo.git", Some("feature/thing"))`.
- `parse_source_ref("https://user:token@github.com/owner/repo.git")` (no trailing ref) still returns `(..., None)` — the credential-URL guard still applies when there is no ref suffix.

### REQ-trust-007

The implementation SHALL meet this contract: `label` returns lowercase string representation

### REQ-trust-008

The implementation SHALL meet this contract: `styled_label` returns colored console output (magenta=local, green=official, cyan=team, yellow=unverified)

## Constraints

- Case-sensitive org matching for `OFFICIAL_ORGS` (handled via duplicate entries: `CorvidLabs`, `corvidlabs`)
- Case-insensitive owner matching for `TEAM_MEMBERS` (GitHub usernames are case-insensitive)
- `OFFICIAL_ORGS` and `TEAM_MEMBERS` are compile-time constants — adding entries requires a code change

## Out of Scope

- Dynamic trust verification (e.g., checking signatures or attestations)
- Trust tier configuration by end users
- Runtime fetching of org/team membership from GitHub APIs
