---
module: introspect
version: 2
status: active
files:
  - src/introspect.rs

db_tables: []
depends_on: []
---

# Introspect

## Purpose

`fledge introspect` dumps the full clap command tree — every subcommand, every argument, every flag — as either a pretty nested listing or as JSON. Lets agents and tooling enumerate the fledge surface without screen-scraping `--help` output. Closes the "how does an agent learn what commands exist?" gap called out in `AGENTS.md`.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point: render the command tree in the requested format |
| `IntrospectOptions` | `{ json: bool }` |
| `CommandNode` | Serializable tree node: name, about, aliases, args, subcommands |
| `ArgNode` | Serializable arg node: name, long, short, help, required, takes_value, value_name, global |
| `INTROSPECT_SCHEMA_VERSION` | `u32` constant — current `--json` schema version, emitted as the top-level `schema_version` field |

### Structs & Enums

| Type | Description |
|------|-------------|
| `IntrospectOptions` | CLI options: `json: bool` |
| `CommandNode` | `{name, about, aliases, args, subcommands}` — recursively nested |
| `ArgNode` | `{name, long?, short?, aliases, help?, required, takes_value, value_name?, global}` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(IntrospectOptions, clap::Command) -> Result<()>` | Build the tree and render it as pretty or JSON |

## Invariants

1. `introspect --json` emits a single JSON object at the top level (never an array), suitable for `jq`/`serde_json::from_str` consumption
2. The top-level object includes a `schema_version: <integer>` field. The current value is `1`. The field is emitted at the same level as `name`, `about`, `args`, and `subcommands` (not nested) so existing consumers that read those keys continue to work — only consumers that need to gate on schema changes read `schema_version`
3. The tree excludes clap's auto-generated `--help` and `--version` args and the `help` subcommand — they're uniform across all commands and would just add noise
4. `value_name` is omitted for boolean flags (args where `takes_value == false`) so agents don't try to pass values where none is expected
5. Global args (clap `global = true`) are emitted only on the command that declared them, with `global: true` to flag their scope — they are NOT mirrored onto every subcommand that inherits them
6. Subcommand aliases (e.g. `plugin` for `plugins`) are surfaced in the `CommandNode.aliases` field; arg-level aliases (both long via `visible_alias` and short via `visible_short_alias`) are surfaced in `ArgNode.aliases`. Agents can therefore recognize both subcommand and flag shorthands (e.g. `--ni` for `--non-interactive`)
7. Without `--json`, the output is a human-readable indented tree: each subcommand nested one level deeper, each arg on its own line with the flag form it takes (`-s, --long` or `<positional>`). Required args are prefixed with `*` as a visual marker
8. `introspect` never touches the filesystem, network, git, or any external tool — it is a pure function of the compiled binary's clap configuration

### Schema Version Compatibility

`schema_version` follows the same additive-only contract as the plugin protocol:

- New top-level or nested fields may be added at any time without bumping `schema_version`. Consumers must ignore unknown fields.
- Removing a field, renaming a field, or changing a field's JSON type is a breaking change and bumps `schema_version` to the next integer.
- Pretty (non-JSON) output is for humans and not version-gated.

## Behavioral Examples

### introspect --json — machine-readable
```
$ fledge introspect --json
{
  "schema_version": 1,
  "name": "fledge",
  "about": "Dev-lifecycle CLI — get your projects ready to fly.",
  "aliases": [],
  "args": [
    {
      "name": "non_interactive",
      "long": "non-interactive",
      "help": "...",
      "required": false,
      "takes_value": false,
      "global": true
    }
  ],
  "subcommands": [
    { "name": "ask", "about": "Ask a question about your codebase", ... },
    { "name": "review", "about": "AI-powered code review of current changes", ... }
  ]
}
```

### introspect — human tree
```
$ fledge introspect
fledge
  Dev-lifecycle CLI — get your projects ready to fly.
  --non-interactive
  ask
    Ask a question about your codebase
    <question>
    --json
    --with-specs <NAMES>
    --no-spec-index
  ...
```

### introspect — aliases surface
```
$ fledge introspect --json | jq '.subcommands[] | select(.aliases | length > 0) | {name, aliases}'
{ "name": "lanes", "aliases": ["lane"] }
{ "name": "plugins", "aliases": ["plugin"] }
{ "name": "templates", "aliases": ["template"] }
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| N/A | This command is a pure read from the in-process clap configuration | No failure modes beyond OS-level write errors to stdout |

## Dependencies

- `clap` — the `Command` / `Arg` introspection API
- `serde` / `serde_json` — serialization of the nested tree

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-23 | Initial spec — `fledge introspect` with pretty and JSON output for agent discoverability |
| 2 | 2026-04-25 | Add `schema_version: 1` to `--json` output (additive — emitted alongside existing top-level keys, not nested). Locks the agent-facing JSON shape for 1.0 |
