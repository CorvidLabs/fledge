# Changelog

fledge can auto-generate changelogs from your git history. You can also view the full project changelog below.

## Using `fledge changelog`

```bash
# Show recent releases
fledge changelog

# Show unreleased changes
fledge changelog --unreleased

# Export as JSON
fledge changelog --json

# Show a specific release
fledge changelog --tag v0.7.0
```

The `changelog` command parses git tags and conventional commits, grouping them by type (features, fixes, etc.).

## Project Changelog

See [CHANGELOG.md](https://github.com/CorvidLabs/fledge/blob/main/CHANGELOG.md) for the full release history.
