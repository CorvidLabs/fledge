---
change: CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks
artifact: docs
---

# Docs

- `site/src/content/docs/reference/cli-reference.md` gains a "Streaming (`--stream`)"
  section covering the intended use for long-running and interactive commands, the
  stderr mirror target, the per-stream ordering guarantee, and the fact that stdin is
  inherited so prompts work.
- `AGENTS.md` extends the existing `fledge run <task> --json` row to mention `--stream`
  and to state that the envelope's `stdout`/`stderr` remain populated, so an agent
  parsing the envelope is unaffected by the flag.
