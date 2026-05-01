---
module: config
version: 10
status: active
files:
  - src/config.rs

db_tables: []
depends_on: []
---

# Config

## Purpose

Manages global user configuration from `~/.config/fledge/config.toml`. Provides default values for author, GitHub org, license, and additional template search paths.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `Config` | Top-level configuration struct with `defaults`, `templates`, `github`, and `ai` sections |
| `Defaults` | Struct holding author, GitHub org, and license default values |
| `TemplatesConfig` | Struct holding additional template directory paths and remote repo references |
| `GitHubConfig` | Struct holding optional GitHub token for authenticated access |
| `AiConfig` | Struct holding LLM provider selection and per-provider settings |
| `ClaudeConfig` | `{ model: Option<String> }` — optional default model passed to `claude --model` |
| `OllamaConfig` | `{ host: String, api_key: Option<String>, model: String, timeout_seconds: u64 }` — endpoint, auth, default model, and per-request timeout |
| `load` | Loads config from disk or returns defaults if file is missing |
| `config_path` | Returns the platform-appropriate path to the config file |
| `save` | Serializes config to TOML and writes to disk, creating parent directories if needed |
| `get` | Returns a config value by dotted key (e.g. `defaults.author`), or `None` if unset/unknown. List keys return newline-separated values |
| `is_secret_key` | Returns whether a dotted key holds a secret value (e.g. `github.token`, `ai.ollama.api_key`) that should be redacted in display |
| `is_valid_key` | Returns whether a dotted key is recognized (scalar or list) |
| `valid_keys_hint` | Returns a static string listing all valid config keys for use in error messages |
| `set` | Sets a scalar config value by dotted key; errors on list keys or unknown keys |
| `unset` | Removes a scalar config value or clears a list config value by dotted key; errors on unknown key |
| `add_to_list` | Appends a value to a list config key (templates.paths, templates.repos); deduplicates; errors on scalar or unknown keys |
| `remove_from_list` | Removes a value from a list config key; returns whether a value was actually removed; errors on scalar or unknown keys |
| `author_or_git` | Returns author from config, falling back to `git config user.name` |
| `github_org` | Returns the configured GitHub org, if any |
| `license` | Returns the configured license, defaulting to "MIT" |
| `extra_template_paths` | Resolves and returns additional template directory paths |
| `github_token` | Returns GitHub token from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN` env var, config, or `gh auth token` CLI fallback |
| `template_repos` | Returns configured remote template repository references |
| `init_config` | Creates a new config file at the default path, optionally with a named preset |

### Structs & Enums

| Type | Description |
|------|-------------|
| `Config` | Top-level config with `defaults`, `templates`, `github`, and `ai` sections |
| `Defaults` | Author, GitHub org, and license defaults |
| `TemplatesConfig` | Additional template directory paths and remote repo references |
| `GitHubConfig` | Optional GitHub token for authenticated access |
| `AiConfig` | Active provider and per-provider settings |
| `ClaudeConfig` | Per-Claude settings: default model override |
| `OllamaConfig` | Per-Ollama settings: host URL, API key, default model, per-request timeout in seconds |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `Config::load` | `() -> Result<Self>` | Load config from disk or return defaults |
| `Config::config_path` | `() -> PathBuf` | Returns path to config file |
| `Config::save` | `(&self) -> Result<()>` | Serialize config to TOML and write to disk, creating parent dirs if needed |
| `Config::get` | `(&self, key: &str) -> Option<String>` | Get a config value by dotted key. List keys return newline-separated values |
| `Config::is_secret_key` | `(key: &str) -> bool` | Returns true for keys that hold secrets (e.g. `github.token`, `ai.ollama.api_key`), used by `config get` to redact output |
| `Config::is_valid_key` | `(key: &str) -> bool` | Check whether a dotted key is recognized |
| `Config::valid_keys_hint` | `() -> &'static str` | Returns a static hint string listing all valid config keys |
| `Config::set` | `(&mut self, key: &str, value: &str) -> Result<()>` | Set a scalar config value, errors on list or unknown keys |
| `Config::unset` | `(&mut self, key: &str) -> Result<()>` | Remove a scalar value or clear a list by dotted key, errors on unknown key |
| `Config::add_to_list` | `(&mut self, key: &str, value: &str) -> Result<()>` | Add a value to a list key with deduplication, errors on scalar or unknown keys |
| `Config::remove_from_list` | `(&mut self, key: &str, value: &str) -> Result<bool>` | Remove a value from a list key, returns whether removed, errors on scalar or unknown keys |
| `Config::author_or_git` | `(&self) -> Option<String>` | Author from config, falling back to `git config user.name` |
| `Config::github_org` | `(&self) -> Option<String>` | GitHub org from config |
| `Config::license` | `(&self) -> String` | License from config, defaulting to "MIT" |
| `Config::extra_template_paths` | `(&self) -> Vec<PathBuf>` | Resolves extra template directory paths |
| `Config::github_token` | `(&self) -> Option<String>` | GitHub token from env vars, config, or `gh` CLI |
| `Config::template_repos` | `(&self) -> &[String]` | Remote template repo references |
| `init_config` | `(Option<&str>) -> Result<()>` | Create config file, optionally applying a named preset |

## Invariants

1. `Config::load` never fails on missing file — returns defaults
2. License always has a value (defaults to "MIT")
3. `~/` prefix in template paths is expanded to the user's home directory
4. GitHub token resolution order: `FLEDGE_GITHUB_TOKEN` env → `GITHUB_TOKEN` env → config file
5. Template repos default to empty list
6. `save` creates parent directories if they don't exist
7. `get`/`set`/`unset` accept dotted keys: `defaults.author`, `defaults.github_org`, `defaults.license`, `github.token`, `templates.paths`, `templates.repos`, `ai.provider`, `ai.claude.model`, `ai.ollama.host`, `ai.ollama.api_key`, `ai.ollama.model`, `ai.ollama.timeout_seconds`
8. `set`/`unset` return an error for unknown keys
9. `set` rejects list keys with guidance to use `add_to_list`
10. `add_to_list`/`remove_from_list` reject scalar keys with guidance to use `set`/`unset`
11. `add_to_list` deduplicates — adding an existing value is a no-op
12. `get` returns newline-separated values for list keys, empty string for empty lists
13. `set("ai.provider", value)` normalizes and validates: only `"claude"` and `"ollama"` are accepted (case-insensitive, trimmed)
14. `unset("ai.ollama.host")` / `unset("ai.ollama.model")` / `unset("ai.ollama.timeout_seconds")` restore the built-in defaults (`http://localhost:11434` / `llama3.3` / `600`) rather than clearing to zero/empty values — those would always fail or hang at request time
15. `OllamaConfig` has a `Default` impl that sets sensible values (local daemon, `llama3.3`, 600s timeout); an `[ai]` section absent from the config file yields the same defaults
16. Absence of an `[ai]` section preserves pre-v0.13 behavior: Claude is the provider with no model override
17. `set("ai.ollama.timeout_seconds", value)` requires a non-negative integer; non-numeric input is rejected with an "Invalid timeout" error

## Behavioral Examples

### Scenario: No config file exists

- **Given** `~/.config/fledge/config.toml` does not exist
- **When** `Config::load()` is called
- **Then** returns `Config::default()` with license="MIT", empty author/org, no extra paths

### Scenario: Author fallback to git

- **Given** config has no `author` field
- **When** `author_or_git()` is called
- **Then** runs `git config user.name` and returns the result

### Scenario: Set a config value

- **Given** config exists with default values
- **When** `set("defaults.author", "Leif")` is called followed by `save()`
- **Then** config file is updated with `author = "Leif"` under `[defaults]`

### Scenario: Get a config value

- **Given** config has `defaults.github_org = "CorvidLabs"`
- **When** `get("defaults.github_org")` is called
- **Then** returns `Some("CorvidLabs")`

### Scenario: Unset a config value

- **Given** config has `defaults.author = "Leif"`
- **When** `unset("defaults.author")` is called followed by `save()`
- **Then** `defaults.author` is `None` in the saved config

### Scenario: Add a template path

- **Given** config has no template paths
- **When** `add_to_list("templates.paths", "/my/templates")` is called followed by `save()`
- **Then** config file has `paths = ["/my/templates"]` under `[templates]`

### Scenario: Add duplicate template path is a no-op

- **Given** config already has `templates.paths = ["/my/templates"]`
- **When** `add_to_list("templates.paths", "/my/templates")` is called
- **Then** list still contains exactly one entry

### Scenario: Remove a template repo

- **Given** config has `templates.repos = ["user/repo", "other/repo"]`
- **When** `remove_from_list("templates.repos", "user/repo")` is called followed by `save()`
- **Then** config has `repos = ["other/repo"]` and returns `true`

### Scenario: Remove nonexistent value returns false

- **Given** config has empty template paths
- **When** `remove_from_list("templates.paths", "/nope")` is called
- **Then** returns `false`, config unchanged

### Scenario: Set on list key errors with guidance

- **Given** any config state
- **When** `set("templates.paths", "/foo")` is called
- **Then** returns error suggesting `add`/`remove` instead

### Scenario: Unknown key error

- **Given** any config state
- **When** `set("invalid.key", "value")` is called
- **Then** returns an error listing valid keys

## Error Cases

| Condition | Behavior |
|-----------|----------|
| Malformed TOML | Returns parse error |
| File not readable | Returns IO error |
| `git config user.name` fails | `author_or_git()` returns `None` |
| Unknown key passed to `set`/`unset` | Returns error with list of valid keys |
| Config directory doesn't exist on `save` | Creates parent directories automatically |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `dirs` | `config_dir()`, `home_dir()` |
| `serde` | Derive `Serialize`, `Deserialize` |
| `toml` | Config file parsing |

### Consumed By

| Module | What is used |
|--------|-------------|
| `init` | `Config::load()`, `extra_template_paths()` |
| `main` | `Config::load()`, `extra_template_paths()` |
| `prompts` | `Config` fields for variable defaults |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 10 | 2026-04-30 | Add `Config::is_secret_key(key) -> bool` — identifies keys whose values should be redacted in display (e.g. `github.token`, `ai.ollama.api_key`). Used by `config get` to avoid printing plaintext secrets to stdout |
| 9 | 2026-04-26 | Document `valid_keys_hint()`, returns a static string listing all valid config keys, used by `handle_config` in main.rs for error messages |
| 8 | 2026-04-24 | Add `ai.ollama.timeout_seconds` scalar (default 600). Mirrors the existing `FLEDGE_AI_TIMEOUT` env var so Ollama timeouts are tunable without env vars; env still wins. `set` parses as `u64` and rejects non-integer input; `unset` restores the 600s default |
| 7 | 2026-04-23 | Add `[ai]` section, `ai.provider` (scalar, `claude`/`ollama` only), `ai.claude.model`, `ai.ollama.host`, `ai.ollama.api_key`, `ai.ollama.model`. Defaults preserve pre-v0.13 Claude-only behavior. `add_to_list`/`remove_from_list` now route through `is_valid_key` so new scalar keys get the right error message |
| 6 | 2026-04-22 | `github_token()` now falls back to `gh auth token` CLI when no env var or config is set |
| 5 | 2026-04-19 | Add `add_to_list()`, `remove_from_list()`, `is_valid_key()`; extend get/set/unset for list keys (`templates.paths`, `templates.repos`) |
| 4 | 2026-04-19 | Add `save()`, `get()`, `set()`, `unset()` for CLI config management |
| 3 | 2026-04-18 | Add `GitHubConfig`, `github_token()`, `template_repos()`, `templates.repos` field |
| 2 | 2026-04-18 | Filled in exported function descriptions, re-validated against source |
| 1 | 2026-04-18 | Initial spec |
