---
module: release
version: 6
status: active
files:
  - src/release/mod.rs
  - src/release/bump.rs
  - src/release/changelog.rs
  - src/release/git.rs
  - src/release/toml_utils.rs
  - src/release/version.rs
  - src/release/tests.rs

db_tables: []
depends_on:
  - versioning
  - changelog
  - lanes
  - run
---

# Release

## Purpose

Provides a unified release workflow: version bumping across language ecosystems, changelog generation from conventional commits, git tagging, and optional push. Supports fledge plugins (`plugin.toml`), Rust, Node/Bun, Python, Ruby, Java (Gradle/Maven), Go, Swift, and any project with git tags.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Execute the full release workflow (preflight → bump → changelog → commit → tag → push) |
| `ReleaseOptions` | Configuration struct for the release command |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ReleaseOptions` | Options: bump level, dry_run, no_tag, no_changelog, no_bump, push, pre_lane, allow_dirty |

## Invariants

1. Preflight checks require a clean working tree (unless `--allow-dirty`) and a git repository
2. Version detection prefers `plugin.toml`'s `[plugin].version` when present (canonical fledge identity), then falls back to language-specific manifests: Cargo.toml, package.json, pyproject.toml, etc.
3. Languages without version files (Go, Swift, generic) use tag-only releases
4. `--dry-run` never modifies any files or creates commits/tags
5. `--no-bump` skips the version-file bump step entirely (tag-only release); useful when the canonical version lives in the GitHub Release tag itself
6. Changelog entries are inserted before existing entries (newest first)
7. The release commit message follows conventional commit format: `chore: release vX.Y.Z`
8. Tags are annotated (`git tag -a`) with message `Release vX.Y.Z`; creating a tag that already exists is rejected with a clear error
9. Custom version files can be specified in `[release].files` section of `fledge.toml`
10. The plugin.toml bumper is scoped to the `[plugin]` table — a `version` key in another table (e.g. a `[[commands]]` row) is left untouched
11. When a Rust plugin carries both `Cargo.toml` and `plugin.toml`, both are bumped in the same release commit so they stay in sync
12. All git commands use explicit `current_dir` for correctness in any working directory context
13. Release has its own `classify_for_changelog()` function that mirrors `changelog::classify_commit()` — same type labels but independent implementations. Both match prefixes case-insensitively, accept breaking `!` markers, and map CorvidLabs-style `Add:` → Features, `Update:` → Changes, `Remove:` → Removals; `strip_conventional_prefix` strips the same prefixes case-insensitively

## Behavioral Examples

### Scenario: Bump patch version in Rust project
```
Given a Cargo.toml with version = "0.8.0"
When fledge release patch
Then Cargo.toml is updated to version = "0.8.1"
And CHANGELOG.md is created/updated with commits since last tag
And a commit "chore: release v0.8.1" is created
And an annotated tag v0.8.1 is created
```

### Scenario: Tag-only release for Go project
```
Given a Go project with no version file
And the latest tag is v1.2.0
When fledge release minor
Then no files are modified
And CHANGELOG.md is created/updated
And tag v1.3.0 is created
```

### Scenario: Dry run
```
Given any project
When fledge release patch --dry-run
Then no files are modified
And no commits or tags are created
And the planned actions are printed
```

### Scenario: Pre-release lane
```
Given a project with a "ci" lane defined
When fledge release minor --pre-lane ci
Then the ci lane runs first
And if it passes, the release proceeds
And if it fails, the release is aborted
```

### Scenario: Custom version files
```
Given fledge.toml contains [release] files = ["version.txt"]
When fledge release patch
Then version.txt is bumped alongside auto-detected files
```

## Error Cases

| Error | Condition |
|-------|-----------|
| Not a git repository | No .git directory in current path |
| Dirty working tree | Uncommitted changes (unless --allow-dirty) |
| No version found | No version file and no git tags |
| Invalid version string | Explicit version doesn't match semver format |
| Pre-lane failure | The specified pre-release lane fails |
| Tag already exists | Target version tag already exists in the repo |
| Git operations fail | Tag creation, commit, or push fails |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `versioning` | `parse_version`, `Version` struct |
| `run` | `detect_project_type` for language detection |
| `lanes` | `LaneAction::Run` for pre-release lanes |
| `regex-lite` | Version pattern matching in files |
| `chrono` | Date formatting for changelog entries |
| `console` | Styled terminal output |
| `toml` | Parsing `[release]` config from fledge.toml |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 6 | 2026-06-11 | `classify_for_changelog` and `strip_conventional_prefix` are now case-insensitive, accept breaking `!` markers, and recognize the CorvidLabs commit style (`Add:` → Features, `Update:` → Changes, `Remove:` → Removals) instead of grouping those commits under Other |
| 5 | 2026-05-01 | **1.0 contract finalize, last-mile fix:** `release --dry-run --json` `files_to_bump` array now includes `[release].files` extras (e.g. `flake.nix`) so the dry-run envelope accurately previews what a real run writes. Previously, `detect_version_files` only looked at the hardcoded language candidates while `bump_version_files` (the real write path) also processed `[release].files` from `fledge.toml` — so dry-run reported `["Cargo.toml"]` while a real run also bumped `flake.nix`. Mirrored the bumper's existence + version-line-regex check in the detection path. Three new tests pin the contract. Caught in independent third-pass review pre-tag |
| 4 | 2026-04-26 | Pre-release lane no longer pollutes `--json` stdout. When `--json` is set, `run_pre_lane` calls a new silent path `crate::lanes::run_for_pre_release(name, dry_run)` that runs steps with subprocess output suppressed and emits no envelope of its own. Fixes a double-envelope regression where `release --json --pre-lane <lane>` previously emitted lane and release envelopes back-to-back on stdout. Failure path unchanged: lane bails with a plain stderr error and exit code 1 |
| 3 | 2026-04-26 | Add `--json` flag. `release X.Y.Z --dry-run --json` emits `{schema_version: 1, action: "release", dry_run: true, version, no_bump, files_to_bump, will_changelog, will_tag, will_push, tag}`. `release X.Y.Z --json` (real run) emits `{schema_version: 1, action: "release", dry_run: false, version, old_version, files_bumped, changelog_updated, commit_created, tag_created, tag, pushed}` and suppresses prose output. Helper functions (`generate_changelog_entry`, `create_release_commit`, `create_tag`, `push_release`) gained a `quiet` param threaded from `opts.json`. New integration tests `cli_release_dry_run_json_emits_envelope` and `cli_release_dry_run_json_no_bump_flag` |
| 2 | 2026-04-25 | Recognize `plugin.toml` (`[plugin].version`) as a first-class fledge-ecosystem version source. Added `--no-bump` flag for tag-only releases. The plugin.toml bumper is section-scoped so other tables' `version` keys (e.g. on `[[commands]]`) aren't touched. Rust plugins with both `Cargo.toml` and `plugin.toml` get both bumped together. (#264) |
| 1 | 2026-04-21 | Initial spec for the full release workflow with multi-language support |
