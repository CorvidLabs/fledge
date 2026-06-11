---
module: changelog
version: 5
status: active
files:
  - src/changelog.rs

db_tables: []
depends_on: []
---

# Changelog

## Purpose

Generate changelogs from git tags and conventional commit messages. Groups commits by type (feat, fix, docs, etc.) and displays them organized by release tag.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — generate and display changelog |
| `ChangelogOptions` | Options: `limit`, `json`, `tag`, `unreleased` |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ChangelogOptions` | Options: `limit`, `json`, `tag`, `unreleased` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(ChangelogOptions) -> Result<()>` | Generate changelog from git history |

## Invariants

1. Lists tags sorted by version (newest first) using `git tag --sort=-version:refname`
2. Groups commits between adjacent tags using conventional commit prefixes
3. Recognizes prefixes case-insensitively: feat, fix, docs, style, refactor, perf, test, build, ci, chore, plus the CorvidLabs-style add (→ Features), update (→ Changes), and remove (→ Removals). `Fix:` and `fix:` classify identically
4. Handles scoped commits like `fix(parser): message`
5. Non-conventional commits are grouped under "Other"
6. `--unreleased` shows commits since the latest tag
7. `--tag` shows a single release
8. `--json` outputs structured JSON
9. Merge commits are excluded via `--no-merges`
10. Breaking change indicators (`!` after type, `BREAKING CHANGE:` footer) are not parsed separately — commits with `!` (`feat!:`, `fix(core)!:`) are classified by their base type (e.g. `feat!:` → Features)

## Behavioral Examples

```
$ fledge changelog
v0.7.0 (2026-04-19)

  Features:
    abc1234 task runner and CI checks

  Fixes:
    def5678 remove invalid --prompt flag

v0.6.0 (2026-04-18)

  Features:
    789abcd distribution & polish

$ fledge changelog --tag v0.7.0
v0.7.0 (2026-04-19)

  Features:
    abc1234 task runner and CI checks

$ fledge changelog --unreleased
Unreleased (2026-04-19)

  Fixes:
    abc1234 fix typo in README

$ fledge changelog --json
{"schema_version": 1, "action": "changelog", "releases": [{"tag": "v0.7.0", "date": "2026-04-19", "sections": [...]}]}
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No tags | Repository has no tags | Info message suggesting `git tag` |
| Tag not found | `--tag` specifies nonexistent tag | Error with tag name |
| Not a git repo | No `.git` directory | Error from git command |

## Dependencies

None (uses only git CLI and standard library)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 5 | 2026-06-11 | Prefix matching is now case-insensitive and understands the CorvidLabs commit style: `Add:` → Features, `Update:` → Changes, `Remove:` → Removals, `Fix:`/`Refactor:`/etc. map to their lowercase categories instead of landing in Other. Breaking `!` markers (`feat!:`, `fix(core)!:`) now classify by base type, matching invariant 10 |
| 4 | 2026-04-26 | Doc sync, behavioral example updated to show the post-tier-D envelope shape (was still showing the pre-1.0 bare-array form). No code change |
| 3 | 2026-04-26 | **Breaking (tier D, 1.0):** `changelog --json` migrated from a bare top-level array to `{schema_version: 1, action: "changelog", releases: [...]}`. Same shape break tier C (#274) applied to the three pillars, this caught a remaining bare-array output. Last-chance shape break before 1.0 freezes the contract. Consumers reading `result[0]` now read `result.releases[0]` |
| 2 | 2026-04-22 | Document that breaking changes are not parsed separately; note no types filter config |
| 1 | 2026-04-19 | Initial spec |
