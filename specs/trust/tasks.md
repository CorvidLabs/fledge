---
spec: trust.spec.md
---

## Tasks

- [x] Extract trust tier logic from `plugin.rs` into shared `trust` module
- [x] Define `TrustTier` enum (Official, Team, Unverified) with serde lowercase serialization
- [x] Implement `label` and `styled_label` methods on `TrustTier`
- [x] Implement `determine_trust_tier` supporting HTTPS, SSH, and shorthand forms
- [x] Implement `determine_trust_tier_from_owner` for direct owner classification
- [x] Implement `parse_source_ref` with credential-URL safety (no false-split on `user:token@host`)
- [x] Wire module into `plugin`, `lanes`, `search`, and `init`
- [x] Write unit tests covering all source forms and every tier label
- [x] Define `TEAM_MEMBERS` allowlist and wire `Team` tier into `determine_trust_tier` / `determine_trust_tier_from_owner`
- [x] Write spec

## Gaps

- `OFFICIAL_ORGS` and `TEAM_MEMBERS` are hardcoded constants; no runtime configuration. Intentional for 1.0 — keeps the trust surface auditable in git history
- No support for non-GitHub sources (GitLab, self-hosted) — all classification assumes GitHub URLs or `owner/repo` shorthand
- A future `Verified` tier for non-CorvidLabs trusted contributors could be added additively without disturbing `Official` or `Team`

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
