---
change: keep-config-init-and-config-command-tests-off-the-operator-config
artifact: context
---

# Context

Issue #447 said `cli_config_init_default` could write a real `~/.config/fledge/config.toml` and asserted nothing. PR #463 pointed that one test at `FLEDGE_CONFIG_DIR` and checked that a file existed. It did not set a disposable `HOME`, did not check the file bytes, and did not keep a sentinel at the path init uses when the override is absent.

A 2026-10-12 reproduction used only disposable homes (`env -i HOME=...`). With `FLEDGE_CONFIG_DIR` set, `config init` wrote that directory and left a sentinel at `$HOME/Library/Application Support/fledge/config.toml` unchanged. Without the override, the same command wrote the home-derived path. A `FLEDGE_CONFIG_DIR` that is a regular file exited 1 with the sentinel unchanged. An existing `config.toml` in the override directory exited 1 and was not rewritten. The same init succeeded under `sandbox-exec` with `(deny network*)`, while a connect to `1.1.1.1:443` under that profile failed.

No production change. `Config::config_path` already prefers `FLEDGE_CONFIG_DIR`. The gap was the test guarantee.

Constraints for anyone picking this up:

- Do not change the config path contract and do not call `std::env::set_var` from these tests.
- Do not read or write the operator config. On Windows, `dirs::config_dir()` ignores `HOME`; those tests must not touch that path.
- Wasmtime lifecycle files and the release-channel docs are outside this change.

Independent Grok review prompted stricter test-helper path validation and persisted remove assertions. Production config behavior and canonical config spec version13 remain unchanged. Root also selects installed/released SpecSync6.1.0 in the existing Trust workflow; this is coordinator-owned delivery metadata, not a new config API or dependency/security qualification. Draft approvals remain empty.
