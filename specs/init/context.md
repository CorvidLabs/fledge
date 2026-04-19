---
spec: init.spec.md
---

## Key Decisions

- `init` is the orchestrator — it delegates to `templates`, `prompts`, `config`, and `remote` modules rather than implementing rendering or prompting itself
- Remote refs (`owner/repo`) are detected early and routed to `run_remote()` for a separate flow that fetches, discovers, and renders from GitHub
- Post-create hooks from remote templates require explicit user confirmation (unless `--yes` is passed) for security
- Git init includes a fallback `user.name`/`user.email` config for CI environments where git identity isn't set
- `--dry-run` shows what would happen (files, hooks, git init) without writing anything
- Target directory must not already exist — no merge/overwrite behavior

## Files to Read First

- `src/init.rs` — orchestration: template resolution, rendering, git init, hooks
- `src/main.rs` — `InitArgs` struct and CLI flag definitions
- `specs/init/init.spec.md` — formal API and invariants

## Current Status

- Full init flow implemented for local and remote templates
- Dry-run, no-git, no-install, refresh, and yes flags all working
- Hook security: remote templates prompt before running hooks
- Post-create summary with file list and next-steps guidance

## Notes

- Template resolution: if `--template` is not given, the user gets an interactive selector via `prompts::select_template()`
- Remote collections (repos with multiple templates) are also supported — the user picks from the discovered set
