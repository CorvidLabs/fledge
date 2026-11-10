---
spec: config.spec.md
---

## Key Decisions

- TOML format for human-readable config at `~/.config/fledge/config.toml`
- Scalar keys (`defaults.author`, `defaults.license`, etc.) use `set`/`unset`; list keys (`templates.paths`, `templates.repos`) use `add`/`remove` — mixing the two is a hard error with guidance
- `author_or_git()` falls back to `git config user.name` when no config author is set
- License defaults to "MIT" when unset (explicit `None` still yields "MIT")
- GitHub token lookup order: `FLEDGE_GITHUB_TOKEN` env → `GITHUB_TOKEN` env → `config.github.token` → `gh auth token` CLI fallback
- Tilde expansion in `templates.paths` maps `~/` to the user's home directory
- `FLEDGE_CONFIG_DIR`, when set, is the directory of `config.toml`. That contract is unchanged. Integration tests set it on the child command together with a disposable `HOME`. They do not mutate the test process environment, so parallel tests cannot point `config_path()` at each other or at the developer config. On Windows, `dirs::config_dir()` is a known-folder path no environment variable redirects, so those tests do not read or write it.

## Files to Read First

- `src/config.rs` — all config structs, load/save/get/set/unset/add/remove logic
- `src/main.rs` — `ConfigAction` enum and `handle_config()` CLI handler
- `specs/config/config.spec.md` — formal API and invariants

## Current Status

- Config CRUD fully implemented (load, save, get, set, unset, add_to_list, remove_from_list)
- CLI subcommands: `config list`, `config get`, `config set`, `config unset`, `config add`, `config remove`
- 40+ unit tests covering all public methods and edge cases
- Spec at v13

## Notes

- `config list` always displays template paths/repos sections (even when empty) for discoverability
- `add_to_list` deduplicates silently; `remove_from_list` returns whether the value was found

Independent Grok review prompted stricter test-helper path validation and persisted remove assertions. Production config behavior and canonical config spec version13 remain unchanged. Root also selects installed/released SpecSync6.1.0 in the existing Trust workflow; this is coordinator-owned delivery metadata, not a new config API or dependency/security qualification. Draft approvals remain empty.
