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
