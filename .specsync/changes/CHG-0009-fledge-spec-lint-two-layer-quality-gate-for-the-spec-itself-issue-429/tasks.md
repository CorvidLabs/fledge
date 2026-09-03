---
change: CHG-0009-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
artifact: tasks
---

# Tasks

- [x] Extract `split_frontmatter` from `parse_frontmatter`; add
      `extract_section_bodies` (`src/spec/parse.rs`)
- [x] Hoist `DEFAULT_REQUIRED_SECTIONS`, add `SPEC_LINT_SCHEMA`, register the
      `lint` submodule, add `SpecAction::Lint` + dispatch (`src/spec/mod.rs`)
- [x] Point `commands::structural_results` at the shared constant
      (`src/spec/commands.rs`)
- [x] Layer 1: `lint_structural` and its pure helpers (`src/spec/lint.rs`)
- [x] Layer 2: prompt, response parsing, check-id normalization, provider
      availability precheck, `grade_spec`, `model_pass_skip_reason`
- [x] Target resolution (module name / path / directory / all)
- [x] `--ignore` parsing, validation against the closed check vocabulary, and
      application
- [x] `build_envelope` (`spec_lint`, `schema_version: 1`) and human reporting
- [x] `SpecSubcommand::Lint` + `spec_action_from` arm (`src/cli.rs`,
      `src/main.rs`)
- [x] Unit tests: one per layer-1 check, every pure helper, layer-2 skip and
      failure paths (stub provider, no network), envelope shape, target
      resolution (`src/spec/tests.rs`)
- [x] CLI tests: exit codes, `--json` envelope, scaffolded spec fails,
      `--ignore`, unknown `--ignore` id (`tests/spec.rs`)
- [x] Update `specs/spec/spec.spec.md` to v14 and `specs/main/main.spec.md` to
      v14
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- [x] Human gate: approve this change definition (`specsync change approve`)

## Deferred decisions (out of scope for this change)

These are open questions for the maintainer, not work items of this change. They are
raised in the PR description and deliberately left unresolved so `spec lint` ships with
the conservative default in each case.

- Whether to add `spec lint` to the `pre-commit` / `ci` lanes in `fledge.toml`. All 33
  specs pass, so wiring it in would be safe, but adding a gate is a maintainer call.
  Shipped un-wired.
- Whether layer 2 should ever default to on. Shipped opt-in via `--ai`; `--no-ai` wins
  when both are passed, so the default can be flipped later without breaking callers.
