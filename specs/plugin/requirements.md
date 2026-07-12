# Plugin — Requirements

## Functional Requirements

1. Install plugins from GitHub shorthand, generic git URLs, and local paths via `fledge plugins install <source>`
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
12. Local path installs are live-linked by default and support `--copy` for snapshot installs
13. Removing a live-linked local plugin must not delete the original local source directory

## Non-Functional Requirements

1. Plugin installation must be idempotent with `--force` flag
2. Plugin binaries must be discoverable via PATH or `plugins/bin/`
3. `--json` flag must produce machine-parseable output for all list/search operations

## Durable Requirements

### REQ-plugin-001

The implementation SHALL satisfy the following criterion: Install plugins from GitHub shorthand, generic git URLs, and local paths via `fledge plugins install <source>`

Acceptance Criteria

- Install plugins from GitHub shorthand, generic git URLs, and local paths via `fledge plugins install <source>`

### REQ-plugin-002

The implementation SHALL satisfy the following criterion: Remove installed plugins via `fledge plugins remove <name>`

Acceptance Criteria

- Remove installed plugins via `fledge plugins remove <name>`

### REQ-plugin-003

The implementation SHALL satisfy the following criterion: List installed plugins with metadata via `fledge plugins list`

Acceptance Criteria

- List installed plugins with metadata via `fledge plugins list`

### REQ-plugin-004

The implementation SHALL satisfy the following criterion: Search for plugins on GitHub via `fledge plugins search <query>`

Acceptance Criteria

- Search for plugins on GitHub via `fledge plugins search <query>`

### REQ-plugin-005

The implementation SHALL satisfy the following criterion: Run plugin commands as fledge subcommands

Acceptance Criteria

- Run plugin commands as fledge subcommands

### REQ-plugin-006

The implementation SHALL satisfy the following criterion: Resolve plugin executables by name for CLI dispatch

Acceptance Criteria

- Resolve plugin executables by name for CLI dispatch

### REQ-plugin-007

The implementation SHALL satisfy the following criterion: Support cross-platform symlink creation (Unix symlinks, Windows symlinks)

Acceptance Criteria

- Support cross-platform symlink creation (Unix symlinks, Windows symlinks)

### REQ-plugin-008

The implementation SHALL satisfy the following criterion: Set executable permissions on plugin binaries (Unix only)

Acceptance Criteria

- Set executable permissions on plugin binaries (Unix only)

### REQ-plugin-009

The implementation SHALL satisfy the following criterion: Scaffold a new plugin via `fledge plugins create <name>` with plugin.toml, bin/, README, and .gitignore

Acceptance Criteria

- Scaffold a new plugin via `fledge plugins create <name>` with plugin.toml, bin/, README, and .gitignore

### REQ-plugin-010

The implementation SHALL satisfy the following criterion: Validate plugin manifests via `fledge plugins validate [path]` — check name, version, binary existence, command definitions

Acceptance Criteria

- Validate plugin manifests via `fledge plugins validate [path]` — check name, version, binary existence, command definitions

### REQ-plugin-011

The implementation SHALL satisfy the following criterion: `publish` validates before pushing

Acceptance Criteria

- `publish` validates before pushing

### REQ-plugin-012

The implementation SHALL satisfy the following criterion: Local path installs are live-linked by default and support `--copy` for snapshot installs

Acceptance Criteria

- Local path installs are live-linked by default and support `--copy` for snapshot installs

### REQ-plugin-013

The implementation SHALL satisfy the following criterion: Removing a live-linked local plugin must not delete the original local source directory

Acceptance Criteria

- Removing a live-linked local plugin must not delete the original local source directory

### REQ-plugin-014

The implementation SHALL satisfy the following criterion: Plugin installation must be idempotent with `--force` flag

Acceptance Criteria

- Plugin installation must be idempotent with `--force` flag

### REQ-plugin-015

The implementation SHALL satisfy the following criterion: Plugin binaries must be discoverable via PATH or `plugins/bin/`

Acceptance Criteria

- Plugin binaries must be discoverable via PATH or `plugins/bin/`

### REQ-plugin-016

The implementation SHALL satisfy the following criterion: `--json` flag must produce machine-parseable output for all list/search operations

Acceptance Criteria

- `--json` flag must produce machine-parseable output for all list/search operations
