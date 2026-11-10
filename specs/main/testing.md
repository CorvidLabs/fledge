Main is tested indirectly through integration tests (`tests/`) which exercise CLI commands end-to-end.

## Config secret output

`tests/config_secret_output.rs` spawns this checkout's `fledge` binary (`CARGO_BIN_EXE_fledge`). Every case gets its own disposable `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`, `FLEDGE_CONFIG_DIR`, and current directory, and removes inherited GitHub and provider credentials from the child. Markers are synthetic and different per key. The test reads only that case's `config.toml`.

| Test | Expectation |
|------|-------------|
| `config_set_masks_github_token` | `config set github.token` exits 0, prints the key, `Set`, and `***`, and omits the marker on both streams. The owned file stores the marker. `config get` prints `***`. `config list` shows a masked row. |
| `config_set_masks_anthropic_api_key` | Same round trip for `ai.anthropic.api_key`. |
| `config_set_masks_openai_api_key` | Same round trip for `ai.openai.api_key`. |
| `config_set_masks_claude_api_key` | Same set/get/persist contract for `ai.claude.api_key`. `config list` does not print the marker. That key has no list row today; a row, if present, must contain `***`. |
| `config_set_masks_ollama_api_key` | Same round trip for `ai.ollama.api_key`. |
| `config_set_prints_nonsecret_defaults_author` | `defaults.author` exits 0, prints the plaintext marker, and persists it. Blanket output suppression fails this test. |
| `config_set_unknown_key_does_not_echo_or_write` | An unknown key exits non-zero, omits the marker on both streams, names `Unknown config key`, and leaves the owned config file absent. |

These are the five keys `Config::is_secret_key` already classifies. The test does not add keys, and it does not cover shell history or process-argument exposure.
