# Versioning — Requirements

### REQ-versioning-001

The implementation SHALL satisfy the behavior described by this companion.

Acceptance Criteria

1. Parse semver strings (MAJOR.MINOR.PATCH)
2. Compare versions correctly (0.2.1 > 0.2.0, 1.0.0 > 0.99.99)
3. Check `min_fledge_version` against compile-time fledge version
4. Return actionable error messages with upgrade instructions
5. Handle optional leading `v` prefix (v1.2.3 → 1.2.3)
