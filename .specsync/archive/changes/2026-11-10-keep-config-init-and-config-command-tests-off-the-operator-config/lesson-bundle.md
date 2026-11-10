# Lesson bundle: keep-config-init-and-config-command-tests-off-the-operator-config

Material for folding this change's lessons into each affected module's lesson file.
Synthesise from what actually happened below; do not restate the change description.

## What this change was

- **Title**: Keep config-init and config command tests off the operator config
- **Kind**: BugFix
- **Specs**: config
- **Paths**: tests/config.rs, tests/common/mod.rs, tests/main.rs, specs/config/testing.md, specs/config/context.md, specs/config/tasks.md, .github/workflows/ci.yml, .github/workflows/trust.yml
- **Acceptance**: Child config commands use disposable HOME and FLEDGE_CONFIG_DIR and write expected default/preset TOML only in the override directory. On Unix, home sentinels remain unchanged, lost-override init proves the real disposable home write target, and the actual sentinel path guard rejects prefix-matching ParentDir traversal plus sibling/relative paths before any write. Windows does not read known-folder paths and checks the explicit override. Non-directory and existing-file destinations fail without modification. macOS init succeeds under actual network denial; supported loopback stand-ins receive no requests. Config remove must exclude the removed value from both CLI output and persisted override TOML. Config production behavior and spec version13 are unchanged. Root-owned Trust workflow selects latest SpecSync6.1.0; no Wasmtime/dependency/security qualification or human approval is implied.

## Evidence

- Verification commit: `89dedac7ce1271a3364d401c97d54ab494ca6f70`
- Base commit: `85d7c4efbca79947afa359504b095c95c486b391`
- Verified by: `specsync check --spec config`

## From the change's context.md

# Context

Issue #447 said `cli_config_init_default` could write a real `~/.config/fledge/config.toml` and asserted nothing. PR #463 pointed that one test at `FLEDGE_CONFIG_DIR` and checked that a file existed. It did not set a disposable `HOME`, did not check the file bytes, and did not keep a sentinel at the path init uses when the override is absent.

A 2026-10-12 reproduction used only disposable homes (`env -i HOME=...`). With `FLEDGE_CONFIG_DIR` set, `config init` wrote that directory and left a sentinel at `$HOME/Library/Application Support/fledge/config.toml` unchanged. Without the override, the same command wrote the home-derived path. A `FLEDGE_CONFIG_DIR` that is a regular file exited 1 with the sentinel unchanged. An existing `config.toml` in the override directory exited 1 and was not rewritten. The same init succeeded under `sandbox-exec` with `(deny network*)`, while a connect to `1.1.1.1:443` under that profile failed.

No production change. `Config::config_path` already prefers `FLEDGE_CONFIG_DIR`. The gap was the test guarantee.

Constraints for anyone picking this up:

- Do not change the config path contract and do not call `std::env::set_var` from these tests.
- Do not read or write the operator config. On Windows, `dirs::config_dir()` ignores `HOME`; those tests must not touch that path.
- Wasmtime lifecycle files and the release-channel docs are outside this change.

Independent Grok review prompted stricter test-helper path validation and persisted remove assertions. Production config behavior and canonical config spec version13 remain unchanged. Root also selects installed/released SpecSync6.1.0 in the existing Trust workflow; this is coordinator-owned delivery metadata, not a new config API or dependency/security qualification. Draft approvals remain empty.


## Exact delivery paths and definition boundary

This draft declares the three changed integration-test files (`tests/config.rs`,
`tests/common/mod.rs`, `tests/main.rs`), the three direct config companions
(`specs/config/testing.md`, `specs/config/context.md`, `specs/config/tasks.md`),
and both changed delivery workflows (`.github/workflows/ci.yml` and
`.github/workflows/trust.yml`). The CI SpecSync binary selector and the Trust
SpecSync selector request 6.1.0. Published workflow bytes, including the
current CI action reference/comment, are preserved by this packet. The exact existing Trust 1.2.1 pin,
verification policy, dependencies and separate Wasmtime work are preserved.

The canonical config spec remains version 13 with no spec-text delta. These are
integration-test guarantees and delivery selectors for its existing
`FLEDGE_CONFIG_DIR` contract. This is a complete definition packet for review;
its draft state, empty approvals/reopenings and absent implementation acceptance
do not provide accepted meaningful-path coverage or lifecycle closure.

## From the change's design.md

# Design

No visual design and no new screen. Config remains a TOML file and a CLI.

The isolation shape is the whole design:

1. Each test builds a `TempEnv` with its own home directory and its own `FLEDGE_CONFIG_DIR`.
2. Those variables are set on the child `Command` only.
3. On Unix, a sentinel file is written at the path `config path` prints when the override is removed. That path has to stay inside the disposable home or the helper refuses to write it. The guard rejects any ParentDir component, as well as relative and outside-prefix paths, before any sentinel write.
4. `config init` is also spawned under `sandbox-exec` `(deny network*)` when that binary exists, and once without the sandbox against a loopback stand-in that must see zero requests.

## From the change's testing.md

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

## Where these lessons go

- `specs/config/context.md`
