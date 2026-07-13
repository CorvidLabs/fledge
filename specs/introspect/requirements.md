---
spec: introspect.spec.md
---

## User Stories

- As an AI agent, I want to enumerate every fledge subcommand and flag programmatically so I can generate correct invocations without screen-scraping `--help` output
- As a tooling author (e.g. someone building a wrapper, a fledge-aware editor integration, or generating docs), I want a single structured source of truth for the command surface that changes automatically as the binary changes
- As a human, I want a bird's-eye view of the whole command tree as a readable indented listing

## Acceptance Criteria

### REQ-introspect-001

The implementation SHALL meet this contract: `fledge introspect --json` produces a single JSON object parseable by `serde_json::from_str` and `jq`

### REQ-introspect-002

The implementation SHALL meet this contract: `fledge introspect` without `--json` produces a human-readable indented tree

### REQ-introspect-003

The implementation SHALL meet this contract: The output includes every user-facing subcommand and arg — no silent gaps

### REQ-introspect-004

The implementation SHALL meet this contract: clap's auto-generated `--help` and `--version` and `help` subcommand are excluded as noise

### REQ-introspect-005

The implementation SHALL meet this contract: Subcommand aliases and global args are explicitly labeled in the output so agents can reason about them

## Constraints

- Must not touch the filesystem, network, or any external process — pure introspection
- Must reflect the *actual* compiled CLI; no separate manifest file that can drift from code
- Must not depend on spec module data (keeps `introspect` independent of whether spec-sync is initialized)

## Out of Scope

- Introspection of plugin-provided subcommands (plugin dispatch happens at runtime via clap's `AllowExternalSubcommand`, outside clap's command graph)
- Documenting default values or value ranges — agents can read those from `--help` if needed; they weren't called out as a pain point
- Any mutation of the CLI (read-only only)
