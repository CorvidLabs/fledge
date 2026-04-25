# Plugin — Requirements

## Functional Requirements

1. Install plugins from GitHub repositories via `fledge plugins install <repo>`
2. Remove installed plugins via `fledge plugins remove <name>`
3. List installed plugins with metadata via `fledge plugins list`
4. Search for plugins on GitHub via `fledge plugins search <query>`
5. Run plugin commands as fledge subcommands
6. Resolve plugin executables by name for CLI dispatch
7. Support cross-platform symlink creation (Unix symlinks, Windows symlinks)
8. Set executable permissions on plugin binaries (Unix only)
9. Scaffold a new plugin via `fledge plugins create <name>` with plugin.toml, bin/, README, and .gitignore
10. Validate plugin manifests via `fledge plugins validate [path]` — check name, version, binary existence, command definitions
11. `publish` validates before pushing

## Non-Functional Requirements

1. Plugin installation must be idempotent with `--force` flag
2. Plugin binaries must be discoverable via PATH or `plugins/bin/`
3. `--json` flag must produce machine-parseable output for all list/search operations
