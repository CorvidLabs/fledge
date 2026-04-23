---
spec: trust.spec.md
---

## User Stories

- As a user installing plugins/templates/lanes, I want to see whether the source is official or unverified so I can make informed trust decisions
- As a module author, I want a shared trust classification function so all extension types use consistent logic

## Acceptance Criteria

- `determine_trust_tier` classifies `CorvidLabs/*` sources as Official
- `determine_trust_tier` classifies all other sources as Unverified
- Supports HTTPS URLs, SSH URLs, and `owner/repo` shorthand
- `parse_source_ref` splits `source@ref` without false-splitting on credential `@` signs
- `label` returns lowercase string representation
- `styled_label` returns colored console output (green=official, cyan=community, yellow=unverified)

## Constraints

- Case-sensitive org matching except `corvidlabs` (lowercase alias)
- Community tier defined but not yet assigned — reserved for future verified-contributor program

## Out of Scope

- Dynamic trust verification (e.g., checking signatures or attestations)
- Trust tier configuration by end users
