---
change: mask-recognized-secrets-in-config-set-confirmation
module: main
---

## ADDED

### REQUIREMENT REQ-main-012

`fledge config set` SHALL persist the exact supplied value for every key `Config::is_secret_key` recognizes (`github.token`, `ai.anthropic.api_key`, `ai.openai.api_key`, `ai.claude.api_key`, `ai.ollama.api_key`) while printing a success confirmation that contains the key and `***` instead of the value, so the supplied secret appears in neither stdout nor stderr. Non-secret keys SHALL still confirm with their plaintext value, and an unknown key SHALL fail without echoing the supplied value or writing the config file.

Acceptance Criteria
- Each of the five recognized secret keys persists byte-identically and confirms with `***`; the marker value appears in neither stdout nor stderr.
- `defaults.author` still prints its plaintext value in the confirmation.
- An unknown key exits non-zero, does not echo the supplied value, and leaves the config file unwritten.
- `config get` and the rendered `config list` keep masking stored secrets.
- The CLI regression suite `tests/config_secret_output.rs` passes all seven cases.

## MODIFIED

### SPEC SECTION Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Unknown command | No matching subcommand or plugin | clap error with suggestions |
| Plugin not found | External command with no matching plugin | Error with `fledge plugins search` hint |
| Unknown config key | `config set` of a key outside `Config::is_valid_key` | Non-zero exit. The supplied value is not printed. The config file is not written |

### SPEC SECTION Invariants

1. All subcommands are defined in the `Commands` enum
2. Unknown commands are forwarded to `plugin::resolve_plugin_command` for plugin pass-through
3. Shell completions support bash, zsh, fish, and PowerShell via `--install` flag
4. The `--version` and `--help` flags are handled by clap
5. The top-level `--non-interactive` flag (aliased `--ni`) is a clap `global = true` arg, available on every subcommand. When set, or when `FLEDGE_NON_INTERACTIVE` env var is truthy (`1`/`true`/`yes`/`y`/`on`), `utils::set_non_interactive(true)` is called before dispatch, so every prompt site in the dispatched command observes it
6. `utils::init_non_interactive_from_env()` runs before `Cli::parse()` so the env var is honored even when users don't pass the flag
7. Tier-B JSON coverage (#271): `templates list` and `templates publish` (handled inline in `main.rs`) honour `--json` and emit a `{schema_version: 1, ...}` envelope on stdout — matching the same envelope contract as `plugins`, `lanes`, `init`, and `create_template`. `templates list --json` returns `{schema_version: 1, templates: [{name, description, source: "builtin"|"local"|"remote", source_ref, path}, ...]}`. `templates publish --json` returns `{schema_version: 1, action: "publish", repo, template, topic, use_hint}` (or `{cancelled: true}` if the user declines an update prompt). Failure paths still exit non-zero in both cases
8. **Empty-list semantics for `templates list`:** when no templates are configured, both modes succeed (exit 0). JSON mode emits `{schema_version: 1, templates: [], hint: "..."}` with the configuration hint as a string field; non-JSON mode prints a friendly "No templates configured" message followed by the same hint. Previously both modes errored with non-zero exit, which forced agents to special-case a list query into an error path
9. **Bare invocation is help, not an error:** when no subcommand is provided, fledge prints the top-level help and exits 0. This matches common CLI conventions and avoids making agents treat a plain `fledge` call as a usage failure
10. **Interactive cursor restoration:** when stdin is a TTY, fledge installs a one-shot Ctrl+C handler that shows the terminal cursor before exiting with code 130, so dialoguer-based prompts (`fledge plugins search --interactive`, template selection, etc.) do not leave the cursor hidden after interruption
11. **Secret `config set` confirmation:** when the key is one `Config::is_secret_key` recognizes (`github.token`, `ai.anthropic.api_key`, `ai.openai.api_key`, `ai.claude.api_key`, `ai.ollama.api_key`), `handle_config` still saves the supplied value and prints a success line containing the key and `***`. The supplied value is absent from that command's stdout and stderr. A key that is not secret still prints its value in the confirmation. `config get` and `config list` continue to mask stored secrets they display
