---
id: keep-config-init-and-config-command-tests-off-the-operator-config
state: draft
type: bug_fix
base_commit: 85d7c4efbca79947afa359504b095c95c486b391
---

# Keep config-init and config command tests off the operator config

## Intent

Keep config-init and config command tests off the operator config

## Affected Canonical Specs

- `config`

## Acceptance Criteria

- Child config commands use disposable HOME and FLEDGE_CONFIG_DIR and write expected default/preset TOML only in the override directory. On Unix, home sentinels remain unchanged, lost-override init proves the real disposable home write target, and the actual sentinel path guard rejects prefix-matching ParentDir traversal plus sibling/relative paths before any write. Windows does not read known-folder paths and checks the explicit override. Non-directory and existing-file destinations fail without modification. macOS init succeeds under actual network denial; supported loopback stand-ins receive no requests. Config remove must exclude the removed value from both CLI output and persisted override TOML. Config production behavior and spec version13 are unchanged. Root-owned Trust workflow selects latest SpecSync6.1.0; no Wasmtime/dependency/security qualification or human approval is implied.

## No-spec Rationale

The config path contract is unchanged. FLEDGE_CONFIG_DIR already selects the config file. This change only proves that contract from child processes with a disposable HOME and records the test boundary in companions.
