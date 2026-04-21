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
