---
change: CHG-0009-fledge-spec-lint-two-layer-quality-gate-for-the-spec-itself-issue-429
artifact: context
---

# Context

## Why now

`fledge spec check` validates code *against* the spec. Nothing validates the
spec. Issue #429 (filed from a four-reader book-club review of the CorvidLabs
books) names the consequence: a thin, aspirational, or poisoned spec passes
every structural check while being wrong, and the spec is the one input to the
agent pipeline that nothing else gates.

The author's follow-up comment on #429 is the load-bearing design constraint:

> the quality judgment should be **agent-graded**, not a static linter. A model
> reviews the spec for quality the way `fledge review` reviews code (is the why
> falsifiable, are invariants real constraints vs decorative, is there an
> acceptance and a rejection signal), with a human able to override. Keep the
> cheap structural checks (sections present, files resolve, valid semver) as a
> fast pre-pass, but "is this spec good" needs a model, not regex.

So the shape is two layers, not one linter.

## Design decisions

1. **Layer 2 is opt-in (`--ai`), not opt-out.** The acceptance criteria in #429
   require the command to work as a pre-commit hook and a CI gate. A default
   that reaches for a provider would make the default path spend money, need
   network, and — with fledge's keyless-local-Ollama default — block on a
   connect timeout in CI. `--no-ai` exists and wins over `--ai` so the default
   can later be flipped without breaking scripted callers.
2. **`version:` accepts an integer *or* semver.** #429 asks for "valid semver",
   but fledge/spec-sync frontmatter uses a monotonic integer (`version: 13`) and
   the typed parser is `u32`. Rejecting integers would fail all 33 of fledge's
   own specs. The check therefore rejects what is neither (`v3`, `1.2`,
   `draft`), and lint normalizes a non-integer value before the typed parse so a
   semver spec still gets its `files:` checks.
3. **"Present" is not enough — sections must be non-empty.** `spec check`
   already asserts the seven headings exist. The failure mode #429 describes is
   a spec that has all seven and says nothing, so `section_has_content` strips
   HTML comments, table header/separator rows, bare bullets, and lone code
   fences before deciding. A freshly scaffolded `spec new` module fails lint —
   that is the intended demonstration.
4. **A gate that could not run must not read as a pass.** A layer-2 provider
   error or unparseable response becomes a `model_pass_failed` **error** finding
   on that spec, not a silent skip. Other specs are still graded.
5. **The human override is `--ignore <check>`**, validated against a closed
   vocabulary so a typo fails loudly instead of suppressing nothing. Model-
   invented check ids collapse to `quality_other` for the same reason.
6. **Only the `.spec.md` goes into the layer-2 prompt**, not the companion
   files. The axes being graded all live in the spec, and a bounded prompt keeps
   a whole-tree pass affordable.
7. **The reachability probe is local to `lint`**, duplicating ~8 lines from
   `doctor` rather than widening `doctor`'s export surface. Different purpose: a
   go/no-go before a long generation, not a diagnostic.

## Related modules

- `spec` — hosts the new `lint` submodule and two new parser helpers.
- `main` — CLI enum variant and dispatch arm.
- `llm` / `config` — provider selection for layer 2, unchanged and shared with
  `review` and `ask`.
- `review` — the model the layer-2 pass is patterned on (prompt construction,
  provider override handling, failure isolation per unit of work).

## Out of scope

- Adding `spec lint` to `fledge.toml`'s `pre-commit` / `ci` lanes. All 33 specs
  pass layer 1 today, so it would be safe, but wiring the gate in is a
  maintainer call.
- A `--fix` mode.
- Persisting or caching layer-2 verdicts between runs.
