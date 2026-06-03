---
spec: plugin.spec.md
---

## Test Plan

### Unit Tests

- Source URL normalization (owner/repo to full URL)
- Source parsing for local paths, generic git URLs, GitHub shorthand, and `--copy` validation
- Plugin name extraction from source
- Plugin manifest TOML parsing
- Safe removal of live-link symlinks without deleting their targets
- resolve_plugin_command finds installed plugins

### Integration Tests

- `fledge plugins list` runs without panic when no plugins installed
- `fledge plugins list --json` outputs valid JSON
- Install/remove cycle works end-to-end with a test plugin
- Scaffolded local plugin installs from `./plugin` without requiring GitHub
- Local plugin update is skipped with a clear status
- Missing plugin produces clear error
