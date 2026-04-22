---
module: plugin
version: 6
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

Plugin system for community extensions. Plugins are external executables that register as fledge subcommands, lane steps, or template post-processors. Discovery uses the same GitHub topic convention as templates (`fledge-plugin`).

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — install, list, remove, or run plugins |
| `PluginOptions` | Options for the plugin subcommand |
| `PluginEntry` | Installed plugin metadata: name, source, version, install date, commands |
| `PluginAction` | Enum of plugin operations: Install, Remove, Update, List, Search, Run |
| `PluginCapabilities` | Declared plugin capabilities: exec, store, metadata |
| `resolve_plugin_command` | Check if a command name matches an installed plugin |
| `run_lifecycle_hook` | Run a named lifecycle hook across all installed plugins |

### Structs & Enums

| Type | Description |
|------|-------------|
| `PluginOptions` | CLI options: `action`, `json` |
| `PluginAction` | Enum: Install, Remove, Update, List, Search, Run |
| `PluginEntry` | Installed plugin record: name, source, version, installed date, commands, pinned_ref |
| `PluginCapabilities` | Declared capabilities — exec, store, metadata (all default false) |
| `PluginManifest` | (private) Parsed `plugin.toml`: name, version, description, commands, hooks |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(PluginOptions) -> Result<()>` | Main entry — dispatch to install/list/remove/run |
| `resolve_plugin_command` | `(&str) -> Option<PathBuf>` | Find plugin executable by command name |
| `run_lifecycle_hook` | `(&str) -> Result<()>` | Run a named lifecycle hook across all installed plugins |

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
binary = "target/release/fledge-deploy"  # relative to plugin dir

[hooks]
build = "cargo build --release"          # runs after clone, before binary check
post_install = "hooks/post-install.sh"   # runs after `fledge plugin install`
post_remove  = "hooks/post-remove.sh"    # runs after `fledge plugin remove`
pre_init = "hooks/pre-init.sh"           # runs before `fledge init`
post_work_start = "hooks/setup-hooks.sh" # runs after `fledge work start`
pre_pr = "hooks/lint-all.sh"             # runs before `fledge work pr` pushes
```

### Build System Auto-Detection

When no `build` hook is specified, fledge auto-detects the build system:

| File | Language | Command |
|------|----------|---------|
| `Cargo.toml` | Rust | `cargo build --release` |
| `Package.swift` | Swift | `swift build -c release` |
| `go.mod` | Go | `go build .` |
| `package.json` | Node | `npm install` |

### Lifecycle Hooks

Plugins can register hooks for lifecycle events beyond install/remove:

| Hook | When | Use Case |
|------|------|----------|
| `pre_init` | Before `fledge init` | Inject custom template variables, validate prerequisites |
| `post_work_start` | After `fledge work start` creates a branch | Set up git hooks, configure branch-specific env |
| `pre_pr` | Before `fledge work pr` pushes and creates PR | Run lint, format, security scans before PR creation |

Lifecycle hooks are called across all installed plugins. Hooks are optional — plugins only participate in events they declare.

### Plugin Discovery

Plugins are discovered via:
1. `~/.config/fledge/plugins/` directory (installed plugins)
2. `PATH` lookup for `fledge-<name>` executables (git-style)
3. GitHub search with `fledge-plugin` topic (for `plugin install`)

### Plugin Installation

`fledge plugin install <repo>[@ref]` clones the repo to `~/.config/fledge/plugins/<name>/`, optionally checks out a pinned git ref (tag, branch, or commit), reads `plugin.toml`, runs the `build` hook (or auto-detects the build system), validates binaries, and symlinks them.

### Version Pinning

Install a specific version with `@ref` syntax: `fledge plugin install owner/repo@v1.2.0`. The ref is stored in `plugins.toml` as `pinned_ref`. Pinned plugins skip `git pull` on update and instead check for newer tags, suggesting an upgrade command if one exists.

## Config Format

Plugins are tracked in `~/.config/fledge/plugins.toml`:

```toml
[[plugins]]
name = "deploy"
source = "someone/fledge-deploy"
version = "0.1.0"
installed = "2026-04-20"

[[plugins]]
name = "fledge-pet"
source = "corvid-agent/fledge-plugin-pet"
version = "0.2.0"
installed = "2026-04-21"
pinned_ref = "v0.2.0"
```

## Invariants

1. Plugins are installed to `~/.config/fledge/plugins/<name>/`
2. Plugin binaries are symlinked to `~/.config/fledge/plugins/bin/`
3. `resolve_plugin_command` checks `plugins/bin/` then PATH for `fledge-<name>`
4. `plugin install` clones the repo, reads `plugin.toml`, runs build hook (or auto-detects), creates symlinks
5. `plugin remove` deletes the plugin directory and its symlinks
6. `plugin update` git pulls and rebuilds unpinned plugins; pinned plugins check for newer tags
7. `plugin list` shows installed plugins with name, version, source, and description
8. `plugin search` uses GitHub topic search (same as template search)
9. Plugin commands appear in `fledge --help` via a "Plugin Commands" section when plugins are installed
10. `--json` outputs structured data for all list/search operations

## Behavioral Examples

```
# Install a plugin from GitHub
$ fledge plugin install someone/fledge-deploy
✅ Installed fledge-deploy v0.1.0
  Commands: deploy

# List installed plugins
$ fledge plugin list
Installed plugins:
  deploy  v0.1.0  Deploy to various cloud providers  (someone/fledge-deploy)

# Run a plugin command
$ fledge deploy staging
▶️ Running plugin: deploy
[plugin output here]

# Remove a plugin
$ fledge plugin remove deploy
✅ Removed fledge-deploy

# Update all plugins
$ fledge plugin update
  ✅ fledge-deploy → v0.2.0
  ✅ fledge-todo → v1.1.0

# Update a specific plugin
$ fledge plugin update fledge-deploy
  ✅ fledge-deploy → v0.2.0

# Search for plugins
$ fledge plugin search deploy
  fledge-deploy   v0.1.0  Deploy to various cloud providers  (someone/fledge-deploy)
  fledge-k8s      v0.3.0  Kubernetes deployment helpers       (other/fledge-k8s)

# Install a specific version
$ fledge plugin install someone/fledge-deploy@v1.2.0
✅ Installed fledge-deploy v1.2.0 (pinned to v1.2.0)
  Commands: deploy

# Update pinned plugin — shows newer tags without changing
$ fledge plugin update fledge-deploy
  * fledge-deploy — pinned to v1.2.0, latest tag is v1.3.0. To upgrade:
    fledge plugin install someone/fledge-deploy@v1.3.0 --force
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Plugin not found | install with invalid repo | Error with suggestion |
| No plugin.toml | Repo missing manifest | Error explaining requirement |
| Already installed | install when present | Error with `--force` suggestion |
| Not installed | remove when absent | Error listing installed plugins |
| Binary not found | plugin.toml references missing binary | Error with build hint |
| Build failed | build hook or auto-detect build fails | Error with toolchain suggestion |
| Ref not found | `@ref` doesn't exist in repo | Error with `git ls-remote` hint |
| Permission denied | Binary not executable | Error with guidance |

## Dependencies

- `config` module (plugin directory paths)
- `github` module (search and clone)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 6 | 2026-04-21 | Fix: add missing Update variant to exported functions table |
| 5 | 2026-04-21 | Add lifecycle hooks: pre_init, post_work_start, pre_pr — run across all installed plugins |
| 4 | 2026-04-21 | Add version pinning with @ref syntax, pinned_ref in registry, smart update for pinned plugins |
| 3 | 2026-04-21 | Add build hook, auto-detect build systems, plugin update command, improved error messages |
| 2 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 1 | 2026-04-20 | Initial spec |
