---
spec: introspect.spec.md
---

## Test Plan

### Unit Tests

Located in `src/introspect.rs`. All use a minimal `TestCli` derived in the tests module so they don't depend on the real `Cli` struct evolving.

- `build_tree_captures_top_level` — root node has correct name and about
- `build_tree_captures_global_args` — clap `global = true` args emit `global: true` in the JSON
- `build_tree_captures_subcommand_with_required_arg` — subcommand's required arg is flagged `required: true`; optional flag is `required: false`
- `build_tree_skips_help_and_version_args` — `--help` and `--version` do not appear in args
- `build_tree_skips_help_subcommand` — `help` subcommand is not in subcommands list
- `tree_serializes_to_valid_json` — `serde_json::to_string` round-trips without error

### Integration Tests

- `fledge introspect --json` on the real fledge binary produces parseable JSON with `name: "fledge"` and a non-empty `subcommands` array
- `fledge introspect --json` includes an entry for every user-facing subcommand (spot-check a few: `ask`, `spec`, `work`, `review`)
- `fledge introspect` (no `--json`) prints a non-empty indented tree and exits 0

### Manual Testing

```bash
# Quick visual inspection
fledge introspect | head -30

# Count subcommands
fledge introspect --json | jq '.subcommands | length'

# List every --json-bearing command
fledge introspect --json | jq -r '.subcommands[] | select(.args[]? | .long == "json") | .name'

# Generate a shell completion-like list
fledge introspect --json | jq -r '.subcommands[].name'
```

### Not tested

- Performance — tree size is bounded by the clap config (~25 subcommands, ~5 args each), sub-millisecond in practice
- Backwards-compat of the JSON schema across fledge versions — schema is explicit in the spec and any change is a version bump of the `introspect` module
