---
spec: trust.spec.md
---

## User Stories

- As a user installing plugins/templates/lanes, I want to see whether the source is local, official, team, or unverified so I can make informed trust decisions
- As a module author, I want a shared trust classification function so all extension types use consistent logic

## Durable Requirements

### REQ-trust-001

The implementation SHALL satisfy the following criterion: `determine_trust_tier` classifies `CorvidLabs/*` sources as Official

Acceptance Criteria

- `determine_trust_tier` classifies `CorvidLabs/*` sources as Official

### REQ-trust-002

The implementation SHALL satisfy the following criterion: `determine_trust_tier` classifies filesystem path sources as Local

Acceptance Criteria

- `determine_trust_tier` classifies filesystem path sources as Local

### REQ-trust-003

The implementation SHALL satisfy the following criterion: `determine_trust_tier` classifies sources owned by a human member of the CorvidLabs org (e.g. `0xLeif/*`) as Team

Acceptance Criteria

- `determine_trust_tier` classifies sources owned by a human member of the CorvidLabs org (e.g. `0xLeif/*`) as Team

### REQ-trust-004

The implementation SHALL satisfy the following criterion: `determine_trust_tier` classifies all other sources as Unverified

Acceptance Criteria

- `determine_trust_tier` classifies all other sources as Unverified

### REQ-trust-005

The implementation SHALL satisfy the following criterion: Supports local paths, HTTPS URLs, SSH URLs, and `owner/repo` shorthand

Acceptance Criteria

- Supports local paths, HTTPS URLs, SSH URLs, and `owner/repo` shorthand

### REQ-trust-006

The implementation SHALL satisfy the following criterion: `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs

Acceptance Criteria

- `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs

### REQ-trust-007

The implementation SHALL satisfy the following criterion: `label` returns lowercase string representation

Acceptance Criteria

- `label` returns lowercase string representation

### REQ-trust-008

The implementation SHALL satisfy the following criterion: `styled_label` returns colored console output (magenta=local, green=official, cyan=team, yellow=unverified)

Acceptance Criteria

- `styled_label` returns colored console output (magenta=local, green=official, cyan=team, yellow=unverified)

## Acceptance Criteria

- `determine_trust_tier` classifies `CorvidLabs/*` sources as Official
- `determine_trust_tier` classifies filesystem path sources as Local
- `determine_trust_tier` classifies sources owned by a human member of the CorvidLabs org (e.g. `0xLeif/*`) as Team
- `determine_trust_tier` classifies all other sources as Unverified
- Supports local paths, HTTPS URLs, SSH URLs, and `owner/repo` shorthand
- `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs
- `label` returns lowercase string representation
- `styled_label` returns colored console output (magenta=local, green=official, cyan=team, yellow=unverified)

## Constraints

- Case-sensitive org matching for `OFFICIAL_ORGS` (handled via duplicate entries: `CorvidLabs`, `corvidlabs`)
- Case-insensitive owner matching for `TEAM_MEMBERS` (GitHub usernames are case-insensitive)
- `OFFICIAL_ORGS` and `TEAM_MEMBERS` are compile-time constants — adding entries requires a code change

## Out of Scope

- Dynamic trust verification (e.g., checking signatures or attestations)
- Trust tier configuration by end users
- Runtime fetching of org/team membership from GitHub APIs
