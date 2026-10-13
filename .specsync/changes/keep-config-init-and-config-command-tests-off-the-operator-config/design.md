---
change: keep-config-init-and-config-command-tests-off-the-operator-config
artifact: design
---

# Design

No visual design and no new screen. Config remains a TOML file and a CLI.

The isolation shape is the whole design:

1. Each test builds a `TempEnv` with its own home directory and its own `FLEDGE_CONFIG_DIR`.
2. Those variables are set on the child `Command` only.
3. On Unix, a sentinel file is written at the path `config path` prints when the override is removed. That path has to stay inside the disposable home or the helper refuses to write it. The guard rejects any ParentDir component, as well as relative and outside-prefix paths, before any sentinel write.
4. `config init` is also spawned under `sandbox-exec` `(deny network*)` when that binary exists, and once without the sandbox against a loopback stand-in that must see zero requests.
