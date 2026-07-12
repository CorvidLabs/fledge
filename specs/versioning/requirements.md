# Versioning — Requirements

1. Parse semver strings (MAJOR.MINOR.PATCH)
2. Compare versions correctly (0.2.1 > 0.2.0, 1.0.0 > 0.99.99)
3. Check `min_fledge_version` against compile-time fledge version
4. Return actionable error messages with upgrade instructions
5. Handle optional leading `v` prefix (v1.2.3 → 1.2.3)

## Durable Requirements

### REQ-versioning-001

The implementation SHALL satisfy the following criterion: Parse semver strings (MAJOR.MINOR.PATCH)

Acceptance Criteria

- Parse semver strings (MAJOR.MINOR.PATCH)

### REQ-versioning-002

The implementation SHALL satisfy the following criterion: Compare versions correctly (0.2.1 > 0.2.0, 1.0.0 > 0.99.99)

Acceptance Criteria

- Compare versions correctly (0.2.1 > 0.2.0, 1.0.0 > 0.99.99)

### REQ-versioning-003

The implementation SHALL satisfy the following criterion: Check `min_fledge_version` against compile-time fledge version

Acceptance Criteria

- Check `min_fledge_version` against compile-time fledge version

### REQ-versioning-004

The implementation SHALL satisfy the following criterion: Return actionable error messages with upgrade instructions

Acceptance Criteria

- Return actionable error messages with upgrade instructions

### REQ-versioning-005

The implementation SHALL satisfy the following criterion: Handle optional leading `v` prefix (v1.2.3 → 1.2.3)

Acceptance Criteria

- Handle optional leading `v` prefix (v1.2.3 → 1.2.3)
