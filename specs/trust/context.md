---
spec: trust.spec.md
---

## Key Decisions

- Extracted from `plugin.rs` so `templates`, `lanes`, and `search` can share one classification rule — trust tiers are a cross-cutting concept, not a plugin-only one
- `TrustTier` serializes as lowercase strings (`"local"`, `"official"`, `"team"`, `"unverified"`) for stable JSON output across commands
- `OFFICIAL_ORGS` and `TEAM_MEMBERS` are `&[&str]` constants, not runtime config — changing either set is a code change and requires a PR. This keeps the trust surface auditable in git history
- `Team` tier (renamed from `Community` in spec v2) classifies personal repos owned by **human members of the CorvidLabs org**. The bar for inclusion is org membership — `TEAM_MEMBERS` is the GitHub username of every human in the org, and any new hire/contributor added to the org should land in the same PR that adds them to this list. As of v2, the list is `["0xGaspar", "0xLeif", "Kyntrin", "tofu-ux"]` — every human in CorvidLabs at that point. The `corvid-agent` org member is intentionally excluded (it is an AI agent, not a human). The list grows in lockstep with org membership. A future verified-contributor program for non-CorvidLabs trusted authors could introduce a separate `Verified` tier without disturbing this one
- `Official` takes precedence over `Team` if an owner ever appears in both lists — defensive ordering, covered by `official_takes_precedence_over_team` test
- `is_team_member` uses `eq_ignore_ascii_case` because GitHub usernames are case-insensitive (`0xLeif` == `0xleif`). `OFFICIAL_ORGS` keeps its case-sensitive `contains` lookup with both casings hardcoded — historical, low-cost to maintain
- `parse_source_ref` uses `rsplit_once('@')` and explicitly guards against credential URLs (`https://user:token@host/...`) and SSH prefixes (`git@github.com:...`) to avoid mis-parsing the `@` as a ref delimiter
- Styled labels use fixed colors (magenta=local, green=official, cyan=team, yellow=unverified) — consistent across every command that shows trust tiers

## Files to Read First

- `src/trust.rs` — complete implementation (~100 LOC + tests)
- `specs/trust/trust.spec.md` — formal API, invariants, and behavioral examples
- `src/plugin/mod.rs` — primary consumer; shows how tiers surface in `list`, `install`, and `audit`
- `src/search.rs`, `src/lanes/mod.rs`, `src/init.rs` — other consumers using `determine_trust_tier_from_owner`

## Current Status

- Fully implemented at v2
- Consumed by 4 modules: `plugin`, `lanes`, `search`, `init`
- Unit tests cover all source forms (local paths, shorthand, HTTPS, SSH, with/without ref, case sensitivity, credential URLs) for Local, Official, Team, and Unverified tiers
- The previously unused `Community` variant has been renamed to `Team` and is now constructed via `TEAM_MEMBERS` membership

## Notes

- If you're adding a new extension type that installs from a source URL, reach for `determine_trust_tier` — do not reinvent the classification
- If you need to surface tier in CLI output, call `styled_label()` for terminal display and `label()` for JSON / structured output
- `parse_source_ref` handles the common `owner/repo@ref` shorthand; if you need to parse the ref separately before classifying, call it first and pass the base to `determine_trust_tier`
- Adding a CorvidLabs team member: append their GitHub username to `TEAM_MEMBERS` in `src/trust.rs` and add a corresponding unit test. Removing a member is the same change in reverse
