---
module: trust
version: 4
status: active
files:
  - src/trust.rs

db_tables: []
depends_on:
  - config
---

# Trust

## Purpose

Shared trust-tier classification for all extension types (plugins, templates, lanes). Determines whether a source is local, official (CorvidLabs org), team (a personal repo owned by a CorvidLabs member or listed in user config), or unverified based on the source form and GitHub org/owner in the source URL or shorthand. Extracted from `plugin.rs` so that templates and lanes can reuse the same logic. Users can extend the team tier at runtime via `trust.orgs` and `trust.users` in `config.toml`.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `TrustTier` | Enum: Local, Official, Team, Unverified — serializes as lowercase strings |
| `label` | Return the trust tier as a lowercase string (TrustTier method) |
| `styled_label` | Return the trust tier as a colored styled string (TrustTier method) |
| `determine_trust_tier` | Classify a source string into a trust tier by parsing the GitHub org/owner |
| `determine_trust_tier_from_owner` | Classify by GitHub owner string directly |
| `parse_source_ref` | Split a source string into base URL and optional git ref (e.g., `@v1.0.0`) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `TrustTier` | Enum: Local, Official, Team, Unverified — serializes as lowercase via serde |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `label` | `(&self) -> &'static str` | Return trust tier as lowercase string ("local", "official", "team", "unverified") |
| `styled_label` | `(&self) -> StyledObject<&'static str>` | Return trust tier as colored console label (magenta/green/cyan/yellow) |
| `determine_trust_tier` | `(&str) -> TrustTier` | Parse source URL/shorthand, extract org/owner, classify tier |
| `determine_trust_tier_from_owner` | `(&str) -> TrustTier` | Classify by owner name directly (no URL parsing) |
| `parse_source_ref` | `(&str) -> (&str, Option<&str>)` | Split `source@ref` into base and optional ref; handles SSH URLs |

## Invariants

1. `CorvidLabs` and `corvidlabs` are the only official orgs
2. `TEAM_MEMBERS` is a hardcoded list of GitHub usernames belonging to **human members of the CorvidLabs org**; their *personal* repos classify as `Team` (sources from the org itself classify as `Official`). The current list is `["0xGaspar", "0xLeif", "Kyntrin", "tofu-ux"]` — every human in the org as of v2 of this spec. The `corvid-agent` org member is excluded (it is an AI agent, not a human). Membership is compared case-insensitively (`0xLeif` and `0xleif` both match)
3. Official tier takes precedence over Team — if an owner appears in both lists (defensive, should not happen), the source classifies as `Official`
4. `determine_trust_tier` supports local paths, HTTPS URLs, SSH URLs, and `owner/repo` shorthand
5. `parse_source_ref` does not split on `@` in credential URLs (e.g., `https://user:token@github.com/...`)
6. `parse_source_ref` handles SSH URLs with `git@` prefix without false-splitting on the prefix `@`
7. Any source not matching a local path, official org, team member, or config entry returns `Unverified`
8. `OFFICIAL_ORGS` and `TEAM_MEMBERS` are `&[&str]` constants providing the baseline trust lists. Users can extend the team tier at runtime via `trust.orgs` and `trust.users` in `config.toml` — these config entries grant **Team** tier only, never Official
9. Config-based orgs and users are compared case-insensitively, matching the behavior of hardcoded `TEAM_MEMBERS`
10. When config cannot be loaded (missing or malformed `config.toml`), the system falls back to an empty `TrustConfig` — only the hardcoded constants apply

## Behavioral Examples

### determine_trust_tier — shorthand
```
determine_trust_tier("CorvidLabs/fledge-plugin-deploy") -> Official
determine_trust_tier("0xLeif/fledge-plugin-thing")     -> Team
determine_trust_tier("someuser/fledge-plugin-thing")    -> Unverified
```

### determine_trust_tier — local paths
```
determine_trust_tier("./fledge-plugin-thing")  -> Local
determine_trust_tier("../fledge-plugin-thing") -> Local
determine_trust_tier("/tmp/fledge-plugin")     -> Local
```

### determine_trust_tier — full URL
```
determine_trust_tier("https://github.com/CorvidLabs/fledge-plugin-deploy") -> Official
determine_trust_tier("https://github.com/0xLeif/fledge-plugin-thing")      -> Team
determine_trust_tier("git@github.com:CorvidLabs/fledge-plugin-deploy.git") -> Official
```

### determine_trust_tier — with ref
```
determine_trust_tier("CorvidLabs/fledge-plugin-deploy@v1.0.0") -> Official
determine_trust_tier("0xLeif/fledge-plugin-thing@v0.1.0")       -> Team
```

### determine_trust_tier — config-extended team tier
```
# Given config.toml contains trust.orgs = ["my-company"] and trust.users = ["corvid-agent"]
determine_trust_tier("my-company/fledge-plugin-foo")    -> Team
determine_trust_tier("corvid-agent/fledge-plugin-bar")  -> Team
determine_trust_tier("My-Company/fledge-plugin-foo")    -> Team  (case-insensitive)
determine_trust_tier("CorvidLabs/fledge-plugin-deploy") -> Official  (hardcoded takes precedence)
```

### parse_source_ref
```
parse_source_ref("someone/fledge-deploy@v1.2.0") -> ("someone/fledge-deploy", Some("v1.2.0"))
parse_source_ref("someone/fledge-deploy") -> ("someone/fledge-deploy", None)
parse_source_ref("https://user:token@github.com/owner/repo.git") -> ("https://user:token@github.com/owner/repo.git", None)
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| N/A | All inputs produce a valid TrustTier | No error cases — defaults to Unverified |

## Dependencies

- `console` — colored terminal output for styled labels
- `serde` — serialization of TrustTier enum
- `crate::config` — `TrustConfig` struct and `Config::load()` for runtime trust extensions

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 4 | 2026-06-03 | Add `Local` trust tier for filesystem path plugin installs. Labels and styled labels include `"local"`; GitHub owner-based classification remains unchanged for search/discovery |
| 3 | 2026-05-03 | Configurable trust: `trust.orgs` and `trust.users` config keys extend the team tier at runtime. Invariant 8 rewritten, invariants 9-10 added. Behavioral examples added for config-driven classification. Depends on `config` module |
| 2 | 2026-04-25 | Rename `Community` → `Team`; add `TEAM_MEMBERS` allowlist of CorvidLabs members (`["0xGaspar", "0xLeif", "Kyntrin", "tofu-ux"]`) classifying their personal repos as `Team`. Drops the unused-variant `#[allow(dead_code)]` since all three tiers now have construction sites |
| 1 | 2026-04-23 | Initial spec, extracted from plugin.rs |
