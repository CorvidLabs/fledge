---
module: main
version: 10
status: active
files:
  - src/main.rs
  - src/cli.rs
  - src/config_cmds.rs
  - src/template_cmds.rs

db_tables: []
depends_on:
  - ai
  - ask
  - changelog
  - config
  - create_template
  - doctor
  - github
  - init
  - introspect
  - lanes
  - llm
  - plugin
  - prompts
  - publish
  - release
  - remote
  - review
  - run
  - search
  - spec
  - spinner
  - templates
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

| Export | Description |
|--------|-------------|
| `Cli` | Top-level clap `#[derive(Parser)]` struct. Holds the `--non-interactive` global flag and the `Commands` subcommand enum |
| `Commands` | Enum of all top-level subcommands: Ai, Ask, Changelog, Completions, Config, Doctor, Introspect, Lanes, Plugins, Release, Review, Run, Spec, Templates, Watch, Work, and External (plugin pass-through) |
| `TemplatesSubcommand` | Enum of `templates` subcommands: Init, Create, Validate, List, Search, Publish |
| `SpecSubcommand` | Enum of `spec` subcommands: Check, Init, List, New, Show |
| `WorkSubcommand` | Enum of `work` subcommands: Start, Pr, Status |
| `AiSubcommand` | Enum of `ai` subcommands: Status, Models, Use |
| `ConfigAction` | Enum of `config` subcommands: Get, Set, Unset, Add, Remove, Edit, List, Path, Init |
| `LaneSubcommand` | Enum of `lanes` subcommands: Run, List, Init, Search, Import, Publish, Create, Validate, and External |
| `PluginSubcommand` | Enum of `plugins` subcommands: Install, Remove, Update, List, Audit, Search, Run, Publish, Create, Validate |
| `handle_config` | Dispatch `fledge config` subcommands (get, set, unset, add, remove, edit, list, path, init) |
| `print_config_described` | Print a single config key with its current value and description |
| `print_config_value_described` | Print a single config key with a non-optional value and description |
| `print_config_list_described` | Print a list config key with its values and description |
| `interactive_config_edit` | Interactive TUI loop for editing config keys via dialoguer prompts |
| `handle_templates` | Dispatch `fledge templates` subcommands (init, create, validate, list, search, publish) |
| `install_completions` | Generate and install shell completions for bash, zsh, or fish |
| `list_templates` | List available templates from configured sources |
| `search_templates` | Search GitHub for community templates by query, author, and limit |
| `publish_template` | Validate and publish a template directory to GitHub with topic tagging |

### Structs & Enums

| Type | Source | Description |
|------|--------|-------------|
| `Cli` | `src/cli.rs` | Top-level clap `#[derive(Parser)]` struct. Holds the `--non-interactive` global flag and the `Commands` subcommand enum |
| `Commands` | `src/cli.rs` | Enum of all top-level subcommands: Ai, Ask, Changelog, Completions, Config, Doctor, Introspect, Lanes, Plugins, Release, Review, Run, Spec, Templates, Watch, Work, and External (plugin pass-through) |
| `TemplatesSubcommand` | `src/cli.rs` | Enum of `templates` subcommands: Init, Create, Validate, List, Search, Publish |
| `SpecSubcommand` | `src/cli.rs` | Enum of `spec` subcommands: Check, Init, List, New, Show |
| `WorkSubcommand` | `src/cli.rs` | Enum of `work` subcommands: Start, Pr, Status |
| `AiSubcommand` | `src/cli.rs` | Enum of `ai` subcommands: Status, Models, Use |
| `ConfigAction` | `src/cli.rs` | Enum of `config` subcommands: Get, Set, Unset, Add, Remove, Edit, List, Path, Init |
| `LaneSubcommand` | `src/cli.rs` | Enum of `lanes` subcommands: Run, List, Init, Search, Import, Publish, Create, Validate, and External |
| `PluginSubcommand` | `src/cli.rs` | Enum of `plugins` subcommands: Install, Remove, Update, List, Audit, Search, Run, Publish, Create, Validate |

### Functions

| Function | Source | Signature | Description |
|----------|--------|-----------|-------------|
| `handle_config` | `src/config_cmds.rs` | `(ConfigAction) -> Result<()>` | Dispatch `fledge config` subcommands (get, set, unset, add, remove, edit, list, path, init) |
| `print_config_described` | `src/config_cmds.rs` | `(&str, &Option<impl Display>, &str)` | Print a single config key with its current value and description |
| `print_config_value_described` | `src/config_cmds.rs` | `(&str, &impl Display, &str)` | Print a single config key with a non-optional value and description |
| `print_config_list_described` | `src/config_cmds.rs` | `(&str, &[String], &str)` | Print a list config key with its values and description |
| `interactive_config_edit` | `src/config_cmds.rs` | `() -> Result<()>` | Interactive TUI loop for editing config keys via dialoguer prompts |
| `handle_templates` | `src/template_cmds.rs` | `(TemplatesSubcommand) -> Result<()>` | Dispatch `fledge templates` subcommands (init, create, validate, list, search, publish) |
| `install_completions` | `src/template_cmds.rs` | `(Option<Shell>) -> Result<()>` | Generate and install shell completions for bash, zsh, or fish |
| `list_templates` | `src/template_cmds.rs` | `(bool) -> Result<()>` | List available templates from configured sources. Bool controls JSON output |
| `search_templates` | `src/template_cmds.rs` | `(Option<&str>, Option<&str>, usize, bool) -> Result<()>` | Search GitHub for community templates by query, author, limit, and JSON flag |
| `publish_template` | `src/template_cmds.rs` | `(&Path, Option<&str>, bool, Option<&str>, bool, bool) -> Result<()>` | Validate and publish a template directory to GitHub with topic tagging |

## Behavioral Examples

```
$ fledge --version
fledge 0.15.2

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
7. Tier-B JSON coverage (#271): `templates list` and `templates publish` (handled inline in `main.rs`) honour `--json` and emit a `{schema_version: 1, ...}` envelope on stdout — matching the same envelope contract as `plugins`, `lanes`, `init`, and `create_template`. `templates list --json` returns `{schema_version: 1, templates: [{name, description, source: "builtin"|"local"|"remote", source_ref, path}, ...]}`. `templates publish --json` returns `{schema_version: 1, action: "publish", repo, template, topic, use_hint}` (or `{cancelled: true}` if the user declines an update prompt). Failure paths still exit non-zero in both cases
8. **Empty-list semantics for `templates list`:** when no templates are configured, both modes succeed (exit 0). JSON mode emits `{schema_version: 1, templates: [], hint: "..."}` with the configuration hint as a string field; non-JSON mode prints a friendly "No templates configured" message followed by the same hint. Previously both modes errored with non-zero exit, which forced agents to special-case a list query into an error path

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 10 | 2026-04-29 | Document all public exports from `cli.rs`, `config_cmds.rs`, and `template_cmds.rs` now that these files are listed in spec frontmatter. No API changes |
| 9 | 2026-04-26 | `templates list` empty case now exits 0 in both modes. JSON mode emits `{schema_version: 1, templates: [], hint}`; non-JSON prints "No templates configured" + hint. Previously both bailed with non-zero exit, breaking agents that call `templates list --json` defensively |
| 8 | 2026-04-25 | **Breaking (tier C, #272):** `templates search --json` migrated from bare top-level array to `{schema_version: 1, results: [...]}`. Last-chance shape break before 1.0 |
| 7 | 2026-04-25 | Tier-B `--json` envelopes for `templates list` and `templates publish` (handled inline in main.rs). Both emit `{schema_version: 1, ...}` matching the contract used by plugins/lanes/init/create_template. Failure paths still exit non-zero. Closes the gap where `templates list --json` previously errored with "unexpected argument" (#271) |
| 5 | 2026-04-23 | Add `llm` to depends_on for the provider abstraction that powers `fledge ask` and `fledge review`. No new top-level command; dispatch changes are localized to the Ask/Review variants. |
| 4 | 2026-04-23 | Add `fledge introspect` command that dumps the clap command tree as JSON or a pretty listing. Closes the "how does an agent learn the command surface?" gap. |
| 3 | 2026-04-23 | Add `--non-interactive` global flag (alias `--ni`) and `FLEDGE_NON_INTERACTIVE` env var. Sets `utils::NON_INTERACTIVE` before dispatch; each subcommand with `--yes`/`--force` auto-promotes it when the flag is set; prompts that have no default bail with a clear error. |
| 2 | 2026-04-23 | Add `watch` to depends_on |
| 1 | 2026-04-21 | Initial spec |
