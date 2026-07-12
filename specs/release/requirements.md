# Release — Requirements

1. Bump version in language-specific files (Cargo.toml, package.json, pyproject.toml, etc.)
2. Support semver bump levels: major, minor, patch, or explicit version
3. Auto-detect project language to find version files
4. Generate/update CHANGELOG.md from conventional commits since last tag
5. Create annotated git tags (vX.Y.Z)
6. Support dry-run mode that shows planned actions without changes
7. Support pre-release lane execution (run CI before releasing)
8. Support tag-only releases for languages without version files (Go, Swift)
9. Allow custom version files via [release] config in fledge.toml
10. Enforce clean working tree (with --allow-dirty override)

## Durable Requirements

### REQ-release-001

The implementation SHALL satisfy the following criterion: Bump version in language-specific files (Cargo.toml, package.json, pyproject.toml, etc.)

Acceptance Criteria

- Bump version in language-specific files (Cargo.toml, package.json, pyproject.toml, etc.)

### REQ-release-002

The implementation SHALL satisfy the following criterion: Support semver bump levels: major, minor, patch, or explicit version

Acceptance Criteria

- Support semver bump levels: major, minor, patch, or explicit version

### REQ-release-003

The implementation SHALL satisfy the following criterion: Auto-detect project language to find version files

Acceptance Criteria

- Auto-detect project language to find version files

### REQ-release-004

The implementation SHALL satisfy the following criterion: Generate/update CHANGELOG.md from conventional commits since last tag

Acceptance Criteria

- Generate/update CHANGELOG.md from conventional commits since last tag

### REQ-release-005

The implementation SHALL satisfy the following criterion: Create annotated git tags (vX.Y.Z)

Acceptance Criteria

- Create annotated git tags (vX.Y.Z)

### REQ-release-006

The implementation SHALL satisfy the following criterion: Support dry-run mode that shows planned actions without changes

Acceptance Criteria

- Support dry-run mode that shows planned actions without changes

### REQ-release-007

The implementation SHALL satisfy the following criterion: Support pre-release lane execution (run CI before releasing)

Acceptance Criteria

- Support pre-release lane execution (run CI before releasing)

### REQ-release-008

The implementation SHALL satisfy the following criterion: Support tag-only releases for languages without version files (Go, Swift)

Acceptance Criteria

- Support tag-only releases for languages without version files (Go, Swift)

### REQ-release-009

The implementation SHALL satisfy the following criterion: Allow custom version files via [release] config in fledge.toml

Acceptance Criteria

- Allow custom version files via [release] config in fledge.toml

### REQ-release-010

The implementation SHALL satisfy the following criterion: Enforce clean working tree (with --allow-dirty override)

Acceptance Criteria

- Enforce clean working tree (with --allow-dirty override)
