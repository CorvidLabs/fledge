---
module: release
version: 1
status: active
files:
  - src/release.rs

db_tables: []
depends_on:
  - versioning
  - changelog
  - lanes
  - run
---

# Release

## Purpose

Provides a unified release workflow: version bumping across language ecosystems, changelog generation from conventional commits, git tagging, and optional push. Supports Rust, Node/Bun, Python, Ruby, Java (Gradle/Maven), Go, Swift, and any project with git tags.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Execute the full release workflow (preflight → bump → changelog → commit → tag → push) |
| `ReleaseOptions` | Configuration struct for the release command |

### Structs & Enums

| Type | Description |
|------|-------------|
| `ReleaseOptions` | Options: bump level, dry_run, no_tag, no_changelog, push, pre_lane, allow_dirty |

## Invariants

1. Preflight checks require a clean working tree (unless `--allow-dirty`) and a git repository
2. Version detection is language-aware: reads from Cargo.toml, package.json, pyproject.toml, etc.
3. Languages without version files (Go, Swift, generic) use tag-only releases
4. `--dry-run` never modifies any files or creates commits/tags
5. Changelog entries are inserted before existing entries (newest first)
6. The release commit message follows conventional commit format: `chore: release vX.Y.Z`
7. Tags are annotated (`git tag -a`) with message `Release vX.Y.Z`; creating a tag that already exists is rejected with a clear error
8. Custom version files can be specified in `[release]` section of `fledge.toml`
9. All git commands use explicit `current_dir` for correctness in any working directory context
10. Release has its own `classify_for_changelog()` function that mirrors `changelog::classify_commit()` — same type labels but independent implementations

## Behavioral Examples

### Scenario: Bump patch version in Rust project
```
Given a Cargo.toml with version = "0.8.0"
When fledge release patch
Then Cargo.toml is updated to version = "0.9.0" (wait, patch)
Actually: version = "0.8.1"
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
| 1 | 2026-04-21 | Initial spec — full release workflow with multi-language support |
