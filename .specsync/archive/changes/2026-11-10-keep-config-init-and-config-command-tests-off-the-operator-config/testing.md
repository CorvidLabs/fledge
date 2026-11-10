---
change: keep-config-init-and-config-command-tests-off-the-operator-config
artifact: testing
---

# Testing

Spawn `fledge` with child environment variables. Do not replace the test process `HOME`.

- `tests/config.rs::cli_config_init_writes_override_preserves_sentinel_and_stays_offline` — confined `config init` writes the exact default `config.toml` in `FLEDGE_CONFIG_DIR`, `config path` prints that file, and Unix home sentinels are unchanged. A second child without the sandbox, pointed at a loopback stand-in for GitHub and Ollama, also succeeds and the stand-in records no request.
- `tests/config.rs::cli_config_init_without_override_writes_the_disposable_home_config` — Unix-only negative control. Unsetting `FLEDGE_CONFIG_DIR` writes the home-derived path and leaves the override directory empty.
- `tests/config.rs::cli_config_init_rejects_a_non_directory_destination` — non-zero status, destination bytes unchanged, Unix home sentinels unchanged.
- `tests/config.rs::cli_config_init_leaves_an_existing_config_unchanged` — non-zero status, existing override bytes unchanged.
- `tests/config.rs::cli_config_init_preset_writes_expected_file_and_preserves_sentinel` — `corvidlabs` preset bytes and an unchanged Unix home sentinel.
- `tests/config.rs::cli_config_path_shows_path` — with the override removed, the printed path stays inside the disposable `HOME` on Unix.
- `tests/main.rs` `cli_config_set_and_get_roundtrip` and `cli_config_add_remove_list_key` — same child `HOME` / `FLEDGE_CONFIG_DIR`, write lands in the override file, Unix home sentinels unchanged.

Unix-only sentinel planting is intentional. Windows known-folder config paths are not redirected, and the tests must not read them.

- `tests/config.rs::cli_config_home_path_guard_rejects_parent_traversal_and_outside_paths` — the actual sentinel guard accepts an inside path and rejects a prefix-matching ParentDir escape, sibling and relative paths without writing any sentinel. It fails the old lexical guard.
- `tests/main.rs::cli_config_add_remove_list_key` — after remove succeeds, both config get output and the persisted override TOML must exclude the removed value.
