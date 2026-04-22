---
module: main
version: 1
status: active
files:
  - src/main.rs

db_tables: []
depends_on:
  - ask
  - changelog
  - checks
  - config
  - create_template
  - deps
  - doctor
  - github
  - init
  - issues
  - lanes
  - metrics
  - plugin
  - prompts
  - prs
  - publish
  - release
  - remote
  - review
  - run
  - search
  - spec
  - spinner
  - templates
  - update
  - validate
  - versioning
  - work
---

# Main

## Purpose

CLI entry point. Defines the top-level `Cli` struct and `Commands` enum using clap derive, parses arguments, and dispatches to the appropriate module. Also handles shell completions generation and plugin command pass-through via clap's `External` variant.

## Public API

### Exported Functions

No public exports — `main.rs` is the binary entry point.

## Behavioral Examples

```
$ fledge --version
fledge 0.8.0

$ fledge --help
Dev-lifecycle CLI — get your projects ready to fly.
[lists all subcommands]

$ fledge completions bash --install
✅ Completions installed for bash

$ fledge unknown-command arg1
▶️ Running plugin: unknown-command
[delegates to plugin if installed, else error]
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Unknown command | No matching subcommand or plugin | clap error with suggestions |
| Plugin not found | External command with no matching plugin | Error with `fledge plugin search` hint |

## Dependencies

All modules are dependencies — main dispatches to every subcommand module. See `depends_on` in frontmatter.

## Invariants

1. All subcommands are defined in the `Commands` enum
2. Unknown commands are forwarded to `plugin::resolve_plugin_command` for plugin pass-through
3. Shell completions support bash, zsh, fish, and PowerShell via `--install` flag
4. The `--version` and `--help` flags are handled by clap

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-21 | Initial spec |
