---
module: changelog
version: 1
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
3. Recognizes prefixes: feat, fix, docs, style, refactor, perf, test, build, ci, chore
4. Handles scoped commits like `fix(parser): message`
5. Non-conventional commits are grouped under "Other"
6. `--unreleased` shows commits since the latest tag
7. `--tag` shows a single release
8. `--json` outputs structured JSON
9. Merge commits are excluded via `--no-merges`

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
[{"tag": "v0.7.0", "date": "2026-04-19", "sections": [...]}]
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
| 1 | 2026-04-19 | Initial spec |
