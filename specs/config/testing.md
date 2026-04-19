---
spec: config.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/config.rs` (inline) | Unit | Default values, TOML parsing, get/set/unset, add/remove, tilde expansion, author fallback, token lookup, key validation |
| `tests/cli_config.rs` | Integration | CLI output for list/get/set/unset/add/remove subcommands (if added) |

## Manual Testing

- [x] `fledge config list` shows all sections including empty template lists
- [x] `fledge config set defaults.author "Test"` persists to config.toml
- [x] `fledge config get defaults.author` prints the value
- [x] `fledge config unset defaults.author` removes the value
- [x] `fledge config add templates.paths /my/templates` adds to list
- [x] `fledge config add templates.paths /my/templates` (duplicate) is a no-op
- [x] `fledge config remove templates.paths /my/templates` removes from list
- [x] `fledge config set templates.paths /foo` errors with guidance to use add/remove
- [x] `fledge config add defaults.author val` errors with guidance to use set
- [x] First config write creates `~/.config/fledge/config.toml` and parent dirs
- [x] Running without a config file uses sensible defaults

## Edge Cases & Boundary Conditions

| Scenario | Expected Behavior |
|----------|-------------------|
| Config file missing | `load()` returns defaults (MIT license, no author, empty lists) |
| Config file empty | Same as missing — all defaults apply |
| Invalid TOML in config file | `load()` returns an error |
| Unknown key in get/set/unset | Error with list of valid keys |
| `unset` on already-None scalar | Silent success (idempotent) |
| `unset` on already-empty list | Silent success (idempotent) |
| `remove_from_list` for value not in list | Returns `Ok(false)`, no error |
| `add_to_list` with duplicate value | Silently deduplicates |
| Tilde path `~/foo` | Expanded to home dir + `/foo` |
| Absolute path `/opt/tpl` | Used as-is, no expansion |
| `license` explicitly set to None | `license()` returns "MIT" |
