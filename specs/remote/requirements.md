# Remote — Requirements

## Functional

- Clone GitHub repos to local cache for template discovery
- Support authenticated access via GitHub token
- Support `owner/repo` and `owner/repo/subpath` reference formats
- Update cached repos on subsequent access
- Shallow clones (`--depth 1`) to minimize bandwidth

## Non-Functional

- Cache location follows platform conventions (XDG on Linux, Library/Caches on macOS)
- Git operations should not leak tokens in stdout/stderr output

## Durable Requirements

### REQ-remote-001

The implementation SHALL satisfy the following criterion: Clone GitHub repos to local cache for template discovery

Acceptance Criteria

- Clone GitHub repos to local cache for template discovery

### REQ-remote-002

The implementation SHALL satisfy the following criterion: Support authenticated access via GitHub token

Acceptance Criteria

- Support authenticated access via GitHub token

### REQ-remote-003

The implementation SHALL satisfy the following criterion: Support `owner/repo` and `owner/repo/subpath` reference formats

Acceptance Criteria

- Support `owner/repo` and `owner/repo/subpath` reference formats

### REQ-remote-004

The implementation SHALL satisfy the following criterion: Update cached repos on subsequent access

Acceptance Criteria

- Update cached repos on subsequent access

### REQ-remote-005

The implementation SHALL satisfy the following criterion: Shallow clones (`--depth 1`) to minimize bandwidth

Acceptance Criteria

- Shallow clones (`--depth 1`) to minimize bandwidth

### REQ-remote-006

The implementation SHALL satisfy the following criterion: Cache location follows platform conventions (XDG on Linux, Library/Caches on macOS)

Acceptance Criteria

- Cache location follows platform conventions (XDG on Linux, Library/Caches on macOS)

### REQ-remote-007

The implementation SHALL satisfy the following criterion: Git operations should not leak tokens in stdout/stderr output

Acceptance Criteria

- Git operations should not leak tokens in stdout/stderr output
