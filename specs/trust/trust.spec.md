---
module: trust
version: 1
status: active
files:
  - src/trust.rs

db_tables: []
depends_on: []
---

# Trust

## Purpose

Shared trust-tier classification for all extension types (plugins, templates, lanes). Determines whether a source is official (CorvidLabs), community, or unverified based on the GitHub org in the source URL or shorthand. Extracted from `plugin.rs` so that templates and lanes can reuse the same logic.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `TrustTier` | Enum: Official, Community, Unverified — serializes as lowercase strings |
| `label` | Return the trust tier as a lowercase string (TrustTier method) |
| `styled_label` | Return the trust tier as a colored styled string (TrustTier method) |
| `determine_trust_tier` | Classify a source string into a trust tier by parsing the GitHub org |
| `determine_trust_tier_from_owner` | Classify by GitHub owner string directly |
| `parse_source_ref` | Split a source string into base URL and optional git ref (e.g., `@v1.0.0`) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `TrustTier` | Enum: Official, Community, Unverified — serializes as lowercase via serde |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `label` | `(&self) -> &'static str` | Return trust tier as lowercase string ("official", "community", "unverified") |
| `styled_label` | `(&self) -> StyledObject<&'static str>` | Return trust tier as colored console label (green/cyan/yellow) |
| `determine_trust_tier` | `(&str) -> TrustTier` | Parse source URL/shorthand, extract org, classify tier |
| `determine_trust_tier_from_owner` | `(&str) -> TrustTier` | Classify by owner name directly (no URL parsing) |
| `parse_source_ref` | `(&str) -> (&str, Option<&str>)` | Split `source@ref` into base and optional ref; handles SSH URLs |

## Invariants

1. `CorvidLabs` and `corvidlabs` are the only official orgs
2. `determine_trust_tier` supports HTTPS URLs, SSH URLs, and `owner/repo` shorthand
3. `parse_source_ref` does not split on `@` in credential URLs (e.g., `https://user:token@github.com/...`)
4. `parse_source_ref` handles SSH URLs with `git@` prefix without false-splitting on the prefix `@`
5. Any source not matching an official org returns `Unverified`
6. `Community` tier is defined but reserved for future use (e.g., verified community contributors)

## Behavioral Examples

### determine_trust_tier — shorthand
```
determine_trust_tier("CorvidLabs/fledge-plugin-deploy") -> Official
determine_trust_tier("someuser/fledge-plugin-thing") -> Unverified
```

### determine_trust_tier — full URL
```
determine_trust_tier("https://github.com/CorvidLabs/fledge-plugin-deploy") -> Official
determine_trust_tier("git@github.com:CorvidLabs/fledge-plugin-deploy.git") -> Official
```

### determine_trust_tier — with ref
```
determine_trust_tier("CorvidLabs/fledge-plugin-deploy@v1.0.0") -> Official
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

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-23 | Initial spec — extracted from plugin.rs |
