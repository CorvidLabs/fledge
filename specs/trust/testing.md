---
spec: trust.spec.md
---

## Test Plan

### Unit Tests

Located in `src/trust.rs` under `#[cfg(test)] mod tests`:

- `official_shorthand` — `CorvidLabs/repo` classifies as `Official`
- `official_full_url` — `https://github.com/CorvidLabs/repo` classifies as `Official`
- `official_ssh_url` — `git@github.com:CorvidLabs/repo.git` classifies as `Official`
- `official_with_ref` — `CorvidLabs/repo@v1.0.0` classifies as `Official` (ref stripped before classification)
- `official_case_insensitive` — lowercase `corvidlabs/repo` classifies as `Official`
- `local_path` — filesystem path sources classify as `Local`
- `team_member_shorthand` — `0xLeif/repo` classifies as `Team`
- `team_member_case_insensitive` — `0xleif/repo` classifies as `Team` (GitHub usernames compare case-insensitively)
- `team_member_full_url` — `https://github.com/0xLeif/repo` classifies as `Team`
- `team_member_with_ref` — `0xLeif/repo@v0.1.0` classifies as `Team` (ref stripped before classification)
- `owner_based_team` — `determine_trust_tier_from_owner("0xLeif")` and `"0xleif"` both return `Team`
- `official_takes_precedence_over_team` — defensive precedence rule: if an owner ever appears in both `OFFICIAL_ORGS` and `TEAM_MEMBERS`, Official wins
- `unverified_third_party` — `someuser/repo` classifies as `Unverified`
- `unverified_full_url` — `https://github.com/someuser/repo` classifies as `Unverified`
- `owner_based_official` — `determine_trust_tier_from_owner("CorvidLabs")` returns `Official`
- `owner_based_unverified` — `determine_trust_tier_from_owner("someuser")` returns `Unverified`
- `parse_source_ref_with_tag` — splits `someone/repo@v1.2.0` into `("someone/repo", Some("v1.2.0"))`
- `parse_source_ref_without_tag` — leaves `someone/repo` unsplit, ref `None`
- `parse_source_ref_full_url_with_tag` — splits `https://github.com/…/repo.git@v2.0.0` correctly
- `parse_source_ref_credential_url_no_split` — credential URL `https://user:token@github.com/…` does NOT split on credential `@`
- `labels` — `label()` returns `"local"`, `"official"`, `"team"`, `"unverified"`

### Integration Tests

Trust classification is exercised end-to-end through the consuming modules:

- `plugin list` / `plugin audit` — trust tier appears in output for each installed plugin (`tests/cli.rs` plugin tests)
- `search` / `lane search` — tier badge appears next to each discovered template/lane
- `init` — official template vs unverified template paths

No dedicated integration test file for `trust` itself — the API surface is pure functions over strings and is covered comprehensively by unit tests.

### Manual Testing

```bash
# Install from an official source and verify badge
fledge plugins install CorvidLabs/fledge-plugin-example
fledge plugins list          # expect green [official]
fledge plugins audit         # expect [official] in audit output

# Install from an unverified source
fledge plugins install someuser/fledge-plugin-example
fledge plugins list          # expect yellow [unverified]

# Search shows tier per result
fledge plugins search deploy
fledge search rust           # templates also show tier

# JSON output uses lowercase labels (post-tier-C envelope: .plugins[])
fledge plugins list --json | jq '.plugins[].trust_tier'
# -> "local" / "official" / "team" / "unverified"
```

### Regression Watch

- Adding a new consumer module: make sure it uses `determine_trust_tier` rather than re-implementing org matching
- Any change to `OFFICIAL_ORGS` or URL parsing logic must be covered by a new unit test before shipping
- Credential URL handling (`parse_source_ref_credential_url_no_split`) is the subtlest case — never remove that test without an explicit replacement
