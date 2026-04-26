---
module: plugin
version: 14
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
| `resolve_plugin_command` | Check if a command name matches an installed plugin |
| `run_lifecycle_hook` | Run a named lifecycle hook across all installed plugins |
| `DEFAULT_PLUGINS` | Curated list of plugin sources installed by `fledge plugins install --defaults` |
| `PluginOptions` | Options for the plugin subcommand |
| `PluginAction` | Enum of plugin operations: Install, Remove, Update, List, Search, Run, Publish, Create, Validate, Audit |
| `PluginEntry` | Installed plugin record (name, source, version, installed date, commands, pinned_ref) |
| `PluginCapabilities` | Declared capabilities — exec, store, metadata (all default false) |

### Structs & Enums

| Type | Description |
|------|-------------|
| `PluginOptions` | CLI options: `action`, `json` |
| `PluginAction` | Enum: Install, Remove, Update, List, Audit, Search, Run, Publish, Create, Validate |
| `PluginEntry` | Installed plugin record: name, source, version, installed date, commands, pinned_ref |
| `PluginCapabilities` | Declared capabilities — exec, store, metadata (all default false) |
| `PluginManifest` | (private) Parsed `plugin.toml`: name, version, description, commands, hooks |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(PluginOptions) -> Result<()>` | Main entry — dispatch to install/list/remove/run/audit |
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
post_install = "hooks/post-install.sh"   # runs after `fledge plugins install`
post_remove  = "hooks/post-remove.sh"    # runs after `fledge plugins remove`
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

### Trust Tiers

Plugins are classified by their source into trust tiers:

| Tier | Criteria | Display |
|------|----------|---------|
| Official | Source org is `CorvidLabs` (case-insensitive) | Green bold `[official]` |
| Team | Source owner is a human member of the CorvidLabs org (`TEAM_MEMBERS` allowlist in `src/trust.rs`) | Cyan `[team]` |
| Unverified | All other sources | Yellow `[unverified]` |

Trust tiers are shown in `plugin list`, `plugin audit`, and during `plugin install`. Unverified plugins with elevated capabilities (exec, metadata) get an extra warning in `plugin audit`.

### Plugin Discovery

Plugins are discovered via:
1. `~/.config/fledge/plugins/` directory (installed plugins)
2. `PATH` lookup for `fledge-<name>` executables (git-style)
3. GitHub search with `fledge-plugin` topic (for `plugin install`)

### Plugin Installation

`fledge plugins install <repo>[@ref]` clones the repo to `~/.config/fledge/plugins/<name>/`, optionally checks out a pinned git ref (tag, branch, or commit), reads `plugin.toml`, runs the `build` hook (or auto-detects the build system), validates binaries, and symlinks them.

### Plugin Runtime Environment

When fledge invokes a plugin command (or runs any of its lifecycle/build hooks, or spawns a fledge-v1 protocol plugin), it sets the following environment variables before exec:

| Variable | Value | Why it exists |
|----------|-------|---------------|
| `FLEDGE_PLUGIN_DIR` | Absolute path to the plugin's source directory (the cloned repo, e.g. `~/.config/fledge/plugins/fledge-plugin-foo`) | The declared `[[commands]].binary` is symlinked into a shared `plugins/bin/` dir, so `dirname "$0"` in a shell plugin resolves to the shared dir, not the plugin's source. `FLEDGE_PLUGIN_DIR` lets a plugin reach sibling helpers, hooks, and fixtures regardless of how it was invoked. |

Plugin authors writing multi-file shell plugins should reference siblings via `"$FLEDGE_PLUGIN_DIR/bin/<helper>"`, not `"$(dirname "$0")/<helper>"`. The `fledge plugins create` scaffold ships a comment + dispatcher example that uses `$FLEDGE_PLUGIN_DIR`.

### Version Pinning

Install a specific version with `@ref` syntax: `fledge plugins install owner/repo@v1.2.0`. The ref is stored in `plugins.toml` as `pinned_ref`. Pinned plugins skip `git pull` on update and instead check for newer tags, suggesting an upgrade command if one exists.

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
7. `plugin list` shows installed plugins with name, version, trust tier, source, and commands
8. `plugin audit` shows trust tier, capabilities, lifecycle hooks, and warnings for each plugin
9. `plugin search` uses GitHub topic search (same as template search)
10. Plugin commands appear in `fledge --help` via a "Plugin Commands" section when plugins are installed
11. `--json` (the global flag at the `plugins` parent command) outputs structured data for **every** plugin subcommand that mutates state or queries the registry: `list`, `audit`, `search`, `install`, `install --defaults`, `remove`, `update`, `update --defaults`, `validate`. Output is a JSON object on stdout with `schema_version: 1` plus an `action` field describing the operation. Prose, spinners, and capability prompts are suppressed in JSON mode (warnings still go to stderr). Errors continue to surface via stderr with non-zero exit code — JSON mode never silently turns a failure into a success exit code. The mutating commands' JSON shape is `{schema_version, action, scope?, installed?|removed?|results?, summary?}`
12. `fledge plugins install --defaults` (mutually exclusive with a positional source ref) installs every entry in the const `DEFAULT_PLUGINS` array. As of v0.15.2: `fledge-plugin-{github,deps,metrics}`. The earlier set also included `templates-remote` (re-absorbed into core `templates search`/`publish`) and `doctor` (re-absorbed into core `doctor` as the informational `Toolchains` section)
13. The `--defaults` install loop reports per-plugin success/failure and continues on error so a single bad repo doesn't block the rest. Exits non-zero if any plugin failed; the trailing summary lists each failure with its error message
14. `fledge plugins update --defaults` (mutually exclusive with a plugin name) updates only the installed plugins from the curated `DEFAULT_PLUGINS` set, matching by source string against either the shorthand (`owner/repo`) or the normalized URL form. Community plugins (e.g. `fledge-plugin-figma`) are left untouched. If none of the defaults are installed, the command suggests `fledge plugins install --defaults` and exits 0
15. `plugins create --json` emits `{schema_version: 1, action: "create", path, name, description, files_created: [...]}`. In JSON mode, interactive prompts are suppressed (yes=true).
16. `plugins publish --json` emits `{schema_version: 1, action: "publish", repo: {owner, name, url}, visibility, validated: bool}`. In JSON mode, interactive prompts are suppressed.

## Behavioral Examples

```
# Install a plugin from GitHub
$ fledge plugins install someone/fledge-deploy
✅ Installed fledge-deploy v0.1.0
  Commands: deploy

# List installed plugins
$ fledge plugins list
Installed plugins:
  deploy  v0.1.0  Deploy to various cloud providers  (someone/fledge-deploy)

# Run a plugin command
$ fledge deploy staging
▶️ Running plugin: deploy
[plugin output here]

# Remove a plugin
$ fledge plugins remove deploy
✅ Removed fledge-deploy

# Update all plugins
$ fledge plugins update
  ✅ fledge-deploy → v0.2.0
  ✅ fledge-todo → v1.1.0

# Update a specific plugin
$ fledge plugins update fledge-deploy
  ✅ fledge-deploy → v0.2.0

# Search for plugins
$ fledge plugins search deploy
  fledge-deploy   v0.1.0  Deploy to various cloud providers  (someone/fledge-deploy)
  fledge-k8s      v0.3.0  Kubernetes deployment helpers       (other/fledge-k8s)

# Install a specific version
$ fledge plugins install someone/fledge-deploy@v1.2.0
✅ Installed fledge-deploy v1.2.0 (pinned to v1.2.0)
  Commands: deploy

# Update pinned plugin — shows newer tags without changing
$ fledge plugins update fledge-deploy
  * fledge-deploy — pinned to v1.2.0, latest tag is v1.3.0. To upgrade:
    fledge plugins install someone/fledge-deploy@v1.3.0 --force

# Scaffold a new plugin
$ fledge plugins create my-tool
✅ Created plugin at ./my-tool

# Validate a plugin
$ fledge plugins validate ./my-tool
✅ my-tool — valid

# Validate with strict mode
$ fledge plugins validate --strict
my-tool
  warn: plugin.author is not set
Validation failed

# Publish runs validation first
$ fledge plugins publish
✅ my-tool — valid
➡️ Publishing plugin ./my-tool as owner/my-tool

# Audit installed plugins
$ fledge plugins audit
Plugin Security Audit

  • fledge-deploy v0.1.0 [official]
    Source: CorvidLabs/fledge-plugin-deploy
    Capabilities:
      • exec — can run shell commands
    Commands: deploy

  • fledge-stats v0.2.0 [unverified]
    Source: someone/fledge-stats
    Capabilities: none
    Commands: stats

  Summary: 2 plugin(s), 1 unverified, 1 with elevated capabilities

# List shows trust tiers
$ fledge plugins list
Installed plugins:
  fledge-deploy  v0.1.0  [official]  (CorvidLabs/fledge-plugin-deploy)
  fledge-stats   v0.2.0  [unverified]  (someone/fledge-stats)
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
| Create dir exists | `create` target directory already exists | Error |
| Validate no plugin.toml | `validate` path missing plugin.toml | Error |
| Validate empty name | `plugin.name` is empty string | Validation error |
| Validate empty version | `plugin.version` is empty string | Validation error |
| Validate missing binary | Command binary doesn't exist and no build hook | Validation error |
| Validate missing binary (build) | Command binary doesn't exist but build hook defined | Validation warning |

## Dependencies

- `config` module (plugin directory paths)
- `github` module (search and clone)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 13 | 2026-04-25 | `--json` now actually emits structured output for `install`, `install --defaults`, `remove`, and `update` (previously the global flag was accepted but silently ignored — agents passing `--json` got ANSI-coloured prose back, a 1.0 footgun caught by the multi-model readiness review). All four emit a `{schema_version: 1, action, ...}` envelope on stdout; warnings stay on stderr; failure paths still exit non-zero so agents can't misclassify them. Invariant 11 rewritten to enumerate the full coverage. |
| 12 | 2026-04-25 | Set `FLEDGE_PLUGIN_DIR` to the plugin's source directory before exec'ing a plugin binary, lifecycle hook, or fledge-v1 protocol plugin. Closes the dogfooding footgun where a multi-file shell plugin's `dirname "$0"` resolved to the shared `plugins/bin/` symlink dir instead of the plugin's source. The `fledge plugins create` scaffold now emits a starter binary that uses `$FLEDGE_PLUGIN_DIR`. (#266) |
| 11 | 2026-04-25 | Trim `DEFAULT_PLUGINS` from 5 entries to 3. `fledge-plugin-templates-remote` was duplicating the in-tree `search.rs`/`publish.rs` helpers in shell — re-absorbed into core (`fledge templates search`/`publish`). `fledge-plugin-doctor` was 110 LOC of shell parallel to core doctor — re-absorbed as the informational `Toolchains` section. Default set is now `{github, deps, metrics}`. |
| 10 | 2026-04-25 | Add `fledge plugins update --defaults` — symmetric with install. Updates only the installed plugins from `DEFAULT_PLUGINS`, leaving community plugins alone. Mutually exclusive with a positional plugin name. |
| 9 | 2026-04-25 | Add `fledge plugins install --defaults` for one-command bulk install of the curated `DEFAULT_PLUGINS` set. Source positional becomes optional when --defaults is used. Per-plugin failures don't abort the bulk install. |
| 8 | 2026-04-23 | Add trust tiers (official/community/unverified) and `audit` subcommand; trust tier shown in list, install, and JSON output |
| 7 | 2026-04-22 | Add `create` and `validate` subcommands; `publish` now validates before pushing |
| 6 | 2026-04-21 | Fix: add missing Update variant to exported functions table |
| 5 | 2026-04-21 | Add lifecycle hooks: pre_init, post_work_start, pre_pr — run across all installed plugins |
| 4 | 2026-04-21 | Add version pinning with @ref syntax, pinned_ref in registry, smart update for pinned plugins |
| 3 | 2026-04-21 | Add build hook, auto-detect build systems, plugin update command, improved error messages |
| 2 | 2026-04-20 | Update behavioral examples to use emojis instead of ASCII/Unicode symbols |
| 1 | 2026-04-20 | Initial spec |
