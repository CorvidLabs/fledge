# Release — Tasks

- [x] Write release spec
- [x] Implement release.rs with full workflow
- [x] Wire into CLI (main.rs)
- [x] Version detection for Rust, Node, Python, Ruby, Java, Go, Swift
- [x] Version file bumping with regex patterns
- [x] Changelog generation from conventional commits
- [x] Git tag creation and optional push
- [x] Dry-run mode
- [x] Pre-release lane support
- [x] Custom version files via [release] config
- [x] 32 unit tests covering all paths

## Gaps

- No `[lanes.release]` in built-in templates yet (users must define their own)
