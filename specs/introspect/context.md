---
spec: introspect.spec.md
---

## Context

`fledge introspect` is the agent-surface capstone: it lets an AI agent or tooling author learn the entire fledge CLI shape in one structured call. Before this, agents had to parse `--help` output or guess command names. With `introspect --json`, every subcommand and flag is machine-readable.

The earlier agent-surface PRs (AGENTS.md, spec list/show, spec-aware ask/review, --non-interactive) made fledge *usable* by agents. This command makes fledge *discoverable* by them.

## Related Modules

- `main` — `main.rs` owns the `Cli` struct; `introspect` is invoked from the main dispatcher with `<Cli as CommandFactory>::command()` as input
- `spec` — similar pattern of "dump a structured view for agents" but over specs rather than commands

## Design Decisions

- **Read from the compiled `clap::Command` rather than a parallel manifest.** Eliminates drift risk: the introspect output can't lie about what the binary accepts. Cost: one more compile-time dep on clap's introspection API.
- **Pretty format exists alongside `--json`.** Humans occasionally want the tree too (e.g. when reviewing which args a subcommand takes) and it's cheap. Default is pretty; `--json` is the agent mode.
- **Help/version args and the `help` subcommand are filtered out.** They're uniform noise — every command has them. Surfacing them in every subcommand's args list would bloat output.
- **`value_name` is suppressed for bool flags.** clap auto-generates uppercase value names even for flags that don't take values. Including them would mislead agents into trying `--json=FOO` or similar.
- **Aliases are surfaced.** `plugin`/`plugins` and `lane`/`lanes` and `template`/`templates` all accept aliases. Agents seeing `plugin` in logs should be able to map back to the canonical `plugins`.
- **No plugin subcommands.** Plugin dispatch happens after clap parses `AllowExternalSubcommand`, so plugins don't appear in the compiled clap graph. Documenting them would require running `fledge plugins list --json` separately — intentional separation of concerns.

## Files to Read First

- `src/introspect.rs` — complete implementation (~180 LOC including tests)
- `specs/introspect/introspect.spec.md` — Public API and invariants
- `src/main.rs` — see the `Commands::Introspect` variant and its dispatcher