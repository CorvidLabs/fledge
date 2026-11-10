# Lesson bundle: mask-recognized-secrets-in-config-set-confirmation

Material for folding this change's lessons into each affected module's lesson file.
Synthesise from what actually happened below; do not restate the change description.

## What this change was

- **Title**: Mask recognized secrets in config set confirmation
- **Kind**: BugFix
- **Specs**: main
- **Paths**: src/config_cmds.rs, tests/config_secret_output.rs, specs/main/main.spec.md, specs/main/testing.md
- **Acceptance**: For every existing Config::is_secret_key key successful config set persists the exact supplied value and confirms with the key plus *** while omitting the supplied secret from stdout and stderr.
- **Acceptance**: Non-secret defaults.author still confirms and persists its plaintext value.
- **Acceptance**: Unknown config keys fail without echoing the supplied value or creating the owned config file.
- **Acceptance**: Existing config get and currently rendered config list secret values stay masked; the absent deprecated Claude list row is not claimed.
- **Acceptance**: The same actual CLI regression changes from five secret-output failures and two passing controls on base to seven passing cases on the candidate.

## Evidence

- Verification commit: `0c0c1787dc0cfc4e1fa743c48f26c489593f2f36`
- Base commit: `85d7c4efbca79947afa359504b095c95c486b391`
- Verified by: `specsync check --spec main`

## From the change's context.md

# Context

This is an agent-prepared UNAPPROVED proposal. It records neither human scope approval nor implementation acceptance, review or signature. It restores existing config secrecy intent and captures no new hi criteria.

On pinned base85d7c4efbca79947afa359504b095c95c486b391, ConfigAction::Set saved its input and printed the raw value, including all five keys already recognized by Config::is_secret_key. Get/List and the interactive secret editor already mask their rendered secret values.

The candidate owns only src/config_cmds.rs, tests/config_secret_output.rs, specs/main/main.spec.md and specs/main/testing.md. It changes the confirmation boundary and preserves exact persistence. PR538's dependency upgrade and PR539/540 scope remain separate. Actual seven-case baseline failed five secret cases and passed two controls; the same target passes all seven after the fix.

Main canonical candidate is already version17 while the pinned base is16. Native workflowv2 creation/answers initially left deltas/ empty. Metadata preparation does not reset canonical bytes, force an additional version or claim materialization. The coordinator must inspect and reconcile the actual delta/version state before later authorized lifecycle advance.

## From the change's design.md

# Design

## Boundary

Preserve load -> set -> save -> confirmation. After save succeeds, select *** when Config::is_secret_key recognizes the key; otherwise display the unchanged supplied string. Only that display selection changes. Save/error behavior, schema, permissions and persistence values are retained.

Deprecated ai.claude.api_key stays classified because existing config still supports it. No operator config is used by the regression fixtures.

## Alternatives

Blanket suppression removes useful feedback and fails the visible non-secret control. Masking stored data breaks scalar persistence and fails exact marker assertions. A duplicated secret-key list risks divergence from Get; the existing classifier is sufficient.

## Security limits

This narrows successful-confirmation stdout exposure. It does not remove process argv/shell history, encrypt storage, audit unrelated commands, or qualify dependencies. The tests run the actual checkout binary with argument arrays and disposable homes/config/cwd; relevant credential variables are removed and all markers are synthetic. Child config operations have no individual timeout, so existing outer gate/CI timeout remains the bound. No network request or signing is required.

## From the change's testing.md

# Testing

## Actual prior evidence

The worker executed these through verification_gate.py F1 and f1_env.py. This metadata preparation does not rerun tests/builds or create lifecycle verification/acceptance records.

| Receipt | Actual result |
|---|---|
| 20261016T070322Z-F1-cfcda0 | Native Fledge offline locked config_secret_output target; gate1/cargo101; seven executed: five real raw-secret-marker failures and two controls pass. |
| 20261016T070640Z-F1-9eb793 | Same target after patch; gate/envelope0; seven pass/zero fail. Passing cases prove exact fake values persist plus masked Get/List. |
| 20261016T070813Z-F1-013eed | pre-commit exit0; fmt/lint, test and spec-check steps success. Envelope omits full-suite count. |
| 20261016T071041Z-F1-cd1d38 | verify-native exit0; all five configured steps success. Three pre-existing recursive self-check skips unchanged. No complete test total exposed. |
| 20261016T070751Z-F1-7ecce5 | SpecSync6.1.1 strict forced coverage exit0:33specs; zero warnings/failures;66/66files and36020/36020LOC. Not lifecycle acceptance. |
| 20261016T070751Z-F1-13c77e | Checkout debug binary main spec lint exit0/version17/zero findings; model pass not requested. Earlier installed1.7.2 unsupported lint exit2. |
| 20261016T071010Z-F1-b88f79 | Cached cargo audit --no-fetch --no-yanked --json exit1:11vulnerabilities and two unmaintained warnings on unchanged Cargo files. No ignore or upgrade. |

## Fixture safety and sensitivity

Tests run the actual Cargo-provided binary, separate TempDirs for HOME/XDG/config/cwd and relevant credential removals, with distinct synthetic markers. They assert exit status/both streams, exact owned TOML persistence, masked Get/List, visible non-secret value and unknown-key non-write. Deprecated Claude's absent List row is disclosed.

Recorded wrapper uses task-local operator/plugin directories, offline cached toolchains and an OS profile denying non-loopback outbound traffic. This is not filesystem confinement or a ban on every loopback service. Config tests require no real credentials, model or live API.

## Pending

Actual cached Trust/risk/provenance, exact nonempty candidate range, live ownership, lifecycle approval/acceptance and hosted checks remain pending. Evidence does not prove argv/history protection, comprehensive credential security, dependency acceptance or release readiness.

## Requirement evidence

Each acceptance criterion of REQ-main-012 in `deltas/main.md`, and what proves it.
Test names are in `tests/config_secret_output.rs`; all seven pass on the materialized tree (`cargo test --test config_secret_output`).

| Requirement | Acceptance criterion | Evidence |
|---|---|---|
| REQ-main-012 | five recognized keys persist byte-identically, confirm with `***`, markers absent from stdout/stderr | `config_set_masks_github_token`, `config_set_masks_anthropic_api_key`, `config_set_masks_openai_api_key`, `config_set_masks_claude_api_key`, `config_set_masks_ollama_api_key` (all via `assert_secret_round_trip`) |
| REQ-main-012 | `defaults.author` still prints its plaintext value | `config_set_prints_nonsecret_defaults_author` |
| REQ-main-012 | unknown key exits non-zero, no echo, config file unwritten | `config_set_unknown_key_does_not_echo_or_write` |
| REQ-main-012 | `config get` and rendered `config list` keep masking stored secrets | `assert_secret_round_trip` get/list assertions inside the five mask tests |
| REQ-main-012 | the regression suite passes all seven cases | `cargo test --test config_secret_output`: 7 passed, 0 failed (also observed on this worktree after rebase onto main b648ea2) |

## Where these lessons go

- `specs/main/context.md`
