---
module: config
version: 4
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
| `Config` | Top-level configuration struct with `defaults`, `templates`, and `github` sections |
| `Defaults` | Struct holding author, GitHub org, and license default values |
| `TemplatesConfig` | Struct holding additional template directory paths and remote repo references |
| `GitHubConfig` | Struct holding optional GitHub token for authenticated access |
| `load` | Loads config from disk or returns defaults if file is missing |
| `config_path` | Returns the platform-appropriate path to the config file |
| `save` | Serializes config to TOML and writes to disk, creating parent directories if needed |
| `get` | Returns a config value by dotted key (e.g. `defaults.author`), or `None` if unset/unknown |
| `set` | Sets a config value by dotted key; errors on unknown key |
| `unset` | Removes a config value by dotted key; errors on unknown key |
| `author_or_git` | Returns author from config, falling back to `git config user.name` |
| `github_org` | Returns the configured GitHub org, if any |
| `license` | Returns the configured license, defaulting to "MIT" |
| `extra_template_paths` | Resolves and returns additional template directory paths |
| `github_token` | Returns GitHub token from `FLEDGE_GITHUB_TOKEN`, `GITHUB_TOKEN` env var, or config |
| `template_repos` | Returns configured remote template repository references |

### Structs & Enums

| Type | Description |
|------|-------------|
| `Config` | Top-level config with `defaults`, `templates`, and `github` sections |
| `Defaults` | Author, GitHub org, and license defaults |
| `TemplatesConfig` | Additional template directory paths and remote repo references |
| `GitHubConfig` | Optional GitHub token for authenticated access |

### Traits

| Trait | Description |
|-------|-------------|

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `Config::load` | `() -> Result<Self>` | Load config from disk or return defaults |
| `Config::config_path` | `() -> PathBuf` | Returns path to config file |
| `Config::save` | `(&self) -> Result<()>` | Serialize config to TOML and write to disk, creating parent dirs if needed |
| `Config::get` | `(&self, key: &str) -> Option<String>` | Get a config value by dotted key (e.g. `defaults.author`) |
| `Config::set` | `(&mut self, key: &str, value: &str) -> Result<()>` | Set a config value by dotted key, errors on unknown key |
| `Config::unset` | `(&mut self, key: &str) -> Result<()>` | Remove a config value by dotted key, errors on unknown key |
| `Config::author_or_git` | `(&self) -> Option<String>` | Author from config, falling back to `git config user.name` |
| `Config::github_org` | `(&self) -> Option<String>` | GitHub org from config |
| `Config::license` | `(&self) -> String` | License from config, defaulting to "MIT" |
| `Config::extra_template_paths` | `(&self) -> Vec<PathBuf>` | Resolves extra template directory paths |
| `Config::github_token` | `(&self) -> Option<String>` | GitHub token from env vars or config |
| `Config::template_repos` | `(&self) -> &[String]` | Remote template repo references |

## Invariants

1. `Config::load` never fails on missing file — returns defaults
2. License always has a value (defaults to "MIT")
3. `~/` prefix in template paths is expanded to the user's home directory
4. GitHub token resolution order: `FLEDGE_GITHUB_TOKEN` env → `GITHUB_TOKEN` env → config file
5. Template repos default to empty list
6. `save` creates parent directories if they don't exist
7. `get`/`set`/`unset` accept dotted keys: `defaults.author`, `defaults.github_org`, `defaults.license`, `github.token`
8. `set`/`unset` return an error for unknown keys

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

| Date | Author | Change |
|------|--------|--------|
| 2026-04-18 | CorvidAgent | Initial spec |
| 2026-04-18 | CorvidAgent | v2: filled in exported function descriptions, re-validated against source |
| 2026-04-18 | CorvidAgent | v3: add GitHubConfig, github_token(), template_repos(), templates.repos field |
| 2026-04-19 | CorvidAgent | v4: add save(), get(), set(), unset() for CLI config management |
