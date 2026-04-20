# Changelog

fledge generates changelogs from your git tags and conventional commits.

## Usage

```bash
fledge changelog                 # recent releases
fledge changelog --unreleased    # changes since last tag
fledge changelog --tag v0.7.0    # specific release
fledge changelog --json          # machine-readable
```

It parses conventional commit messages and groups them by type (features, fixes, etc.).

## Full Changelog

See [CHANGELOG.md](https://github.com/CorvidLabs/fledge/blob/main/CHANGELOG.md) for the complete release history.
