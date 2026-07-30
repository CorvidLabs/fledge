---
change: CHG-0007-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
artifact: docs
---

# Docs

## Canonical specs (updated in this change)

- `specs/spec/spec.spec.md` → v14. Purpose, exports, invariants 16–24,
  behavioral examples for `spec lint` (clean, scaffolded, `--json`, `--ai`,
  `--ai` with no provider, `--ignore`), error cases, dependencies, change log.
- `specs/main/main.spec.md` → v14. `SpecSubcommand` variant list.

## Agent-facing surface (`CLAUDE.md` / `AGENTS.md`)

The `--json` table gains a row. Suggested wording for whoever lands the doc
pass:

> `fledge spec lint [target] --json` — `{schema_version: 1, action:
> "spec_lint", strict, model_pass: {requested, ran, skipped_reason, provider,
> model}, specs: [{name, path, version, status, findings: [{check, severity,
> layer, section, message}], errors, warnings, ignored, passed}], totals:
> {linted, errors, warnings, ignored}, passed}`. Layer 1 (structural) always
> runs offline; layer 2 (model-graded) is opt-in via `--ai`. Use before handing
> a spec to an agent loop.

`spec lint` also belongs in the "Before reporting a task done" section next to
`fledge spec check`, and `--ignore` should be described as the human override.

## Human docs (`site/src/content/docs/`)

Not updated in this change. The spec command page needs a `spec lint` section
covering the two layers, the check-id vocabulary, and why the model pass is
opt-in. Tracked as follow-up, not a blocker for the code gate.

## Not changed

- `README.md` — the command list there is intentionally short.
- `fledge.toml` lanes — see the open maintainer decision in `tasks.md`.
