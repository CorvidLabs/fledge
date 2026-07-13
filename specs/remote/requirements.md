# Remote — Requirements

## Functional

### REQ-remote-001

The implementation SHALL meet this contract: Clone GitHub repos to local cache for template discovery

### REQ-remote-002

The implementation SHALL meet this contract: Support authenticated access via GitHub token

### REQ-remote-003

The implementation SHALL meet this contract: Support `owner/repo` and `owner/repo/subpath` reference formats

### REQ-remote-004

The implementation SHALL meet this contract: Update cached repos on subsequent access

### REQ-remote-005

The implementation SHALL meet this contract: Shallow clones (`--depth 1`) to minimize bandwidth

## Non-Functional

### REQ-remote-006

The implementation SHALL meet this contract: Cache location follows platform conventions (XDG on Linux, Library/Caches on macOS)

### REQ-remote-007

The implementation SHALL meet this contract: Git operations should not leak tokens in stdout/stderr output
