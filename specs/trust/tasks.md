---
spec: trust.spec.md
---

## Tasks

- [x] Extract trust tier logic from `plugin.rs` into shared `trust` module
- [x] Define `TrustTier` enum (Official, Community, Unverified) with serde lowercase serialization
- [x] Implement `label` and `styled_label` methods on `TrustTier`
- [x] Implement `determine_trust_tier` supporting HTTPS, SSH, and shorthand forms
- [x] Implement `determine_trust_tier_from_owner` for direct owner classification
- [x] Implement `parse_source_ref` with credential-URL safety (no false-split on `user:token@host`)
- [x] Wire module into `plugin`, `lanes`, `search`, and `init`
- [x] Write unit tests covering all source forms and the community tier label
- [x] Write spec

## Gaps

- `Community` tier is defined and styled but no source currently maps to it — reserved for a future verified-contributor program
- Org matching is limited to a hardcoded `OFFICIAL_ORGS` list (`CorvidLabs`, `corvidlabs`); no runtime configuration
- No support for non-GitHub sources (GitLab, self-hosted) — all classification assumes GitHub URLs or `owner/repo` shorthand

## Review Sign-offs

- **Product**: done
- **QA**: done
- **Design**: n/a
- **Dev**: done
