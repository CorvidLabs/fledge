---
spec: plugin.spec.md
---

## Test Plan

### Unit Tests

- Source URL normalization (owner/repo to full URL)
- Plugin name extraction from source
- Plugin manifest TOML parsing
- resolve_plugin_command finds installed plugins

### Integration Tests

- `fledge plugin list` runs without panic when no plugins installed
- `fledge plugin list --json` outputs valid JSON
- Install/remove cycle works end-to-end with a test plugin
- Missing plugin produces clear error
