---
spec: trust.spec.md
---

## Key Decisions

- Extracted from `plugin.rs` so `templates`, `lanes`, and `search` can share one classification rule — trust tiers are a cross-cutting concept, not a plugin-only one
- `TrustTier` serializes as lowercase strings (`"official"`, `"community"`, `"unverified"`) for stable JSON output across commands
- `OFFICIAL_ORGS` is a `&[&str]` constant, not runtime config — changing the official set is a code change and should require a PR
- `Community` tier is defined now even though no source maps to it, so downstream code (list, audit, search) already handles three cases and adding community sources later is additive
- `parse_source_ref` uses `rsplit_once('@')` and explicitly guards against credential URLs (`https://user:token@host/...`) and SSH prefixes (`git@github.com:...`) to avoid mis-parsing the `@` as a ref delimiter
- Styled labels use fixed colors (green=official, cyan=community, yellow=unverified) — consistent across every command that shows trust tiers

## Files to Read First

- `src/trust.rs` — complete implementation (~100 LOC + tests)
- `specs/trust/trust.spec.md` — formal API, invariants, and behavioral examples
- `src/plugin.rs` — primary consumer; shows how tiers surface in `list`, `install`, and `audit`
- `src/search.rs`, `src/lanes.rs`, `src/init.rs` — other consumers using `determine_trust_tier_from_owner`

## Current Status

- Fully implemented at v1
- Consumed by 4 modules: `plugin`, `lanes`, `search`, `init`
- 13 unit tests covering all source forms (shorthand, HTTPS, SSH, with/without ref, case sensitivity, credential URLs)
- No known bugs or open work items

## Notes

- If you're adding a new extension type that installs from a source URL, reach for `determine_trust_tier` — do not reinvent the classification
- If you need to surface tier in CLI output, call `styled_label()` for terminal display and `label()` for JSON / structured output
- `parse_source_ref` handles the common `owner/repo@ref` shorthand; if you need to parse the ref separately before classifying, call it first and pass the base to `determine_trust_tier`
- The `#[allow(dead_code)]` on `TrustTier` is intentional: not every variant is constructed from every call site, but all three are public API surface
