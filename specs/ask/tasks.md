---
spec: ask.spec.md
---

## Tasks

- [x] Write spec files for ask module
- [x] Implement argument joining for natural question input
- [x] Implement Claude CLI invocation with question
- [x] Add empty-question validation
- [x] Wire up CLI subcommand in main.rs
- [x] Write unit tests
- [x] Run verification suite
- [x] Default-on spec index (compact purpose per module, feeds into every `ask`)
- [x] `--with-specs <names>` flag (comma/repeat, `"all"` expansion)
- [x] `--no-spec-index` escape hatch
- [x] Unit tests for `build_prompt`, `expand_with_specs`, and prompt composition
- [x] CLI test that `ask --help` advertises the new flags

## Gaps

- The compact index uses the first paragraph of `## Purpose`; a module whose Purpose opens with a list or table gets a weaker summary
- No caching — every `ask` invocation re-reads and re-renders all 32 specs. Not a bottleneck today but something to revisit if the prompt prefix grows
- Companions are included whole; for huge `context.md` files this could balloon the prompt. A future pass could section-filter companions
