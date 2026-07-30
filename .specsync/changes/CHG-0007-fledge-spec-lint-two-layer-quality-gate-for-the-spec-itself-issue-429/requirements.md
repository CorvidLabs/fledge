---
change: CHG-0007-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
artifact: requirements
---

# Requirements

## User stories

- As a maintainer, I want a gate that blocks a spec that is structurally
  complete but says nothing, so a scaffolded or hollowed-out spec cannot become
  the input to an agent loop.
- As an agent, I want `fledge spec lint --json` to tell me exactly which checks
  a spec failed, with stable ids, so I can fix the spec rather than guess.
- As a CI operator, I want the default invocation to be offline, deterministic,
  and free, so `spec lint` can sit in a pre-commit hook and a CI job.
- As a reviewer, I want a model's judgment on whether a spec's "why" is
  falsifiable and its invariants are load-bearing, because no regex can answer
  that.
- As a human, I want to override the agent's judgment on a specific check
  without disabling the whole gate.

## Acceptance criteria

1. `fledge spec lint [target]` exits `0` when clean and `1` with a structured
   list of failures otherwise. `target` accepts a module name, a `.spec.md`
   path, or a directory; omitted, it lints every spec.
2. Layer 1 always runs, performs no network I/O, and covers: all required
   sections present and non-empty; no `TODO`/`TBD`/`FIXME` in Purpose or Public
   API; a `version:` that is an integer or semver; every `files:` entry present
   on disk; at least one acceptance signal and one rejection signal (non-empty
   Behavioral Examples / Error Cases, or explicit `accepts:` / `rejects:`
   frontmatter blocks).
3. Layer 2 runs only under `--ai`, is skipped with a stated reason otherwise,
   and grades falsifiable purpose, load-bearing invariants, acceptance signal,
   rejection signal, and internal consistency.
4. Requesting layer 2 with no usable provider produces a clear error naming the
   env var to set and the escape hatch — never a hang.
5. `--json` emits `{schema_version: 1, action: "spec_lint", strict, model_pass,
   specs: [...], totals, passed}`; errors go to stderr as plain text even under
   `--json`; the exit code is the contract.
6. `--strict` promotes warnings to errors. `--ignore <check>` suppresses named
   checks and counts them in `ignored`; an unknown id is an error.
7. `FLEDGE_NON_INTERACTIVE=1` / `--non-interactive` is respected — `spec lint`
   never prompts.

## Constraints

- Reuse the existing spec parser (`src/spec/parse.rs`); no second parser.
- Reuse the existing provider plumbing (`src/llm.rs`, `Config`); no new
  provider abstraction.
- Only *add* an envelope call site (`envelope::action`); do not refactor
  existing ones.
- No test may make a real network call to an LLM.

## Out of scope

- Wiring `spec lint` into the repo's own lanes.
- Auto-fixing findings.
- Grading companion files (requirements/tasks/context/testing).
