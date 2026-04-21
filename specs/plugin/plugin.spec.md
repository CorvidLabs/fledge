---
module: plugin
version: 1
status: active
files:
  - src/plugin.rs

db_tables: []
depends_on:
  - config
  - github
---

# Plugin

## Purpose

Plugin system for community extensions. Plugins are external executables that register as fledge subcommands, flow steps, or template post-processors. Discovery uses the same GitHub topic convention as templates (`fledge-plugin`).

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — install, list, remove, or run plugins |
| `PluginOptions` | Options for the plugin subcommand |
| `PluginEntry` | Installed plugin metadata: name, source, version, install date, commands |
| `PluginAction` | Enum of plugin operations: Install, Remove, List, Search, Run |
| `resolve_plugin_command` | Check if a command name matches an installed plugin |
| `list_installed` | List all installed plugins with metadata |

### Structs & Enums

| Type | Description |
|------|-------------|
| `PluginOptions` | CLI options: `action`, `json` |
| `PluginAction` | Enum: Install, Remove, List, Search, Run |
| `PluginEntry` | Installed plugin record: name, source, version, installed date, commands |
| `PluginManifest` | (private) Parsed `plugin.toml`: name, version, description, commands |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(PluginOptions) -> Result<()>` | Main entry — dispatch to install/list/remove/run |
| `resolve_plugin_command` | `(&str) -> Option<PathBuf>` | Find plugin executable by command name |
| `list_installed` | `() -> Result<Vec<PluginEntry>>` | List all installed plugins |

## Plugin Format

Plugins are git repositories containing a `plugin.toml` manifest and one or more executables:

```toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
description = "Deploy to various cloud providers"
author = "someone"

[[commands]]
name = "deploy"
description = "Deploy the project"
binary = "fledge-deploy"  # relative to plugin dir
```

### Plugin Discovery

Plugins are discovered via:
1. `~/.config/fledge/plugins/` directory (installed plugins)
2. `PATH` lookup for `fledge-<name>` executables (git-style)
3. GitHub search with `fledge-plugin` topic (for `plugin install`)

### Plugin Installation

`fledge plugin install <repo>` clones the repo to `~/.config/fledge/plugins/<name>/`, reads `plugin.toml`, and symlinks binaries.

## Config Format

Plugins are tracked in `~/.config/fledge/plugins.toml`:

```toml
[[plugins]]
name = "deploy"
source = "github:someone/fledge-deploy"
version = "0.1.0"
installed = "2026-04-20"
```

## Invariants

1. Plugins are installed to `~/.config/fledge/plugins/<name>/`
2. Plugin binaries are symlinked to `~/.config/fledge/plugins/bin/`
3. `resolve_plugin_command` checks `plugins/bin/` then PATH for `fledge-<name>`
4. `plugin install` clones the repo, reads `plugin.toml`, creates symlinks
5. `plugin remove` deletes the plugin directory and its symlinks
6. `plugin list` shows installed plugins with name, version, source, and description
7. `plugin search` uses GitHub topic search (same as template search)
8. Plugin commands appear in `fledge --help` via a "Plugin Commands" section when plugins are installed
9. `--json` outputs structured data for all list/search operations

## Behavioral Examples

```
# Install a plugin from GitHub
$ fledge plugin install someone/fledge-deploy
✓ Installed fledge-deploy v0.1.0
  Commands: deploy

# List installed plugins
$ fledge plugin list
Installed plugins:
  deploy  v0.1.0  Deploy to various cloud providers  (someone/fledge-deploy)

# Run a plugin command
$ fledge deploy staging
▸ Running plugin: deploy
[plugin output here]

# Remove a plugin
$ fledge plugin remove deploy
✓ Removed fledge-deploy

# Search for plugins
$ fledge plugin search deploy
  fledge-deploy   v0.1.0  Deploy to various cloud providers  (someone/fledge-deploy)
  fledge-k8s      v0.3.0  Kubernetes deployment helpers       (other/fledge-k8s)
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Plugin not found | install with invalid repo | Error with suggestion |
| No plugin.toml | Repo missing manifest | Error explaining requirement |
| Already installed | install when present | Error with `--force` suggestion |
| Not installed | remove when absent | Error listing installed plugins |
| Binary not found | plugin.toml references missing binary | Error during install |
| Permission denied | Binary not executable | Error with guidance |

## Dependencies

- `config` module (plugin directory paths)
- `github` module (search and clone)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-20 | Initial spec |
