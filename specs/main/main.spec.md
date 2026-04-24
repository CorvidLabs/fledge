---
module: main
version: 4
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
  - introspect
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
  - watch
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
| Plugin not found | External command with no matching plugin | Error with `fledge plugins search` hint |

## Dependencies

All modules are dependencies — main dispatches to every subcommand module. See `depends_on` in frontmatter.

## Invariants

1. All subcommands are defined in the `Commands` enum
2. Unknown commands are forwarded to `plugin::resolve_plugin_command` for plugin pass-through
3. Shell completions support bash, zsh, fish, and PowerShell via `--install` flag
4. The `--version` and `--help` flags are handled by clap
5. The top-level `--non-interactive` flag (aliased `--ni`) is a clap `global = true` arg, available on every subcommand. When set, or when `FLEDGE_NON_INTERACTIVE` env var is truthy (`1`/`true`/`yes`/`y`/`on`), `utils::set_non_interactive(true)` is called before dispatch, so every prompt site in the dispatched command observes it
6. `utils::init_non_interactive_from_env()` runs before `Cli::parse()` so the env var is honored even when users don't pass the flag

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 4 | 2026-04-23 | Add `fledge introspect` command that dumps the clap command tree as JSON or a pretty listing. Closes the "how does an agent learn the command surface?" gap. |
| 3 | 2026-04-23 | Add `--non-interactive` global flag (alias `--ni`) and `FLEDGE_NON_INTERACTIVE` env var. Sets `utils::NON_INTERACTIVE` before dispatch; each subcommand with `--yes`/`--force` auto-promotes it when the flag is set; prompts that have no default bail with a clear error. |
| 2 | 2026-04-23 | Add `watch` to depends_on |
| 1 | 2026-04-21 | Initial spec |
