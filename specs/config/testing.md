---
spec: config.spec.md
---

## Automated Testing

| Test File | Type | What It Covers |
|-----------|------|----------------|
| `src/config.rs` (inline) | Unit | Default values, TOML parsing, get/set/unset, add/remove, tilde expansion, author fallback, token lookup, key validation |
| `tests/config.rs` | Integration | CLI list/get/set, and `config init` against a child `HOME` plus `FLEDGE_CONFIG_DIR` |
| `tests/main.rs` | Integration | `config set`/`get` and `config add`/`remove` round-trips in that same child environment |
| `tests/common/mod.rs` | Helper | `TempEnv` sets the child environment. It does not call `std::env::set_var` |

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

Sentinel planting, preservation and lost-override home-path controls run only on Unix. Windows checks verify the explicit override file and do not read known-folder config paths. macOS proves actual network denial with sandbox-exec; other platforms use the existing loopback stand-in.

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
| Unix: `FLEDGE_CONFIG_DIR` set on the child, sentinel at the home-derived config path | `config init` writes only `$FLEDGE_CONFIG_DIR/config.toml` and leaves the sentinel bytes unchanged |
| Unix: `FLEDGE_CONFIG_DIR` unset, disposable `HOME` | `config init` writes the home-derived config path (the sentinel's location) |
| `FLEDGE_CONFIG_DIR` is a regular file | `config init` exits non-zero and does not modify that file or Unix home sentinel bytes |
| `config.toml` already exists in the override directory | `config init` exits non-zero and leaves those bytes unchanged |
| Network denied (`sandbox-exec` on macOS) and GitHub/Ollama pointed at a loopback stand-in | `config init` still succeeds and the stand-in receives no request |

The sentinel path guard rejects ParentDir traversal, sibling and relative printed paths before any write. Its regression deliberately uses a path that passes the old lexical HOME prefix but resolves outside it. The list remove round-trip rereads both CLI output and persisted TOML and requires the removed value to be absent.
