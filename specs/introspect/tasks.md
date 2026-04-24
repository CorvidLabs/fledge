---
spec: introspect.spec.md
---

## Tasks

- [x] Design `CommandNode` and `ArgNode` with serde `Serialize`
- [x] Implement recursive `build_tree` from `clap::Command`
- [x] Filter out `help` / `version` auto-generated args and `help` subcommand
- [x] Suppress `value_name` for boolean flags (`takes_value == false`)
- [x] Pretty renderer (indented tree with flag forms and aliases)
- [x] Wire `Commands::Introspect { json }` in main.rs and add `mod introspect;`
- [x] Write unit tests against a minimal `#[derive(Parser)]` test CLI
- [x] Write spec + companions

## Gaps

- Default values are not in the tree — agents that need them still have to read `--help`. Adding them would require threading through clap's `get_default_values` and formatting for each arg type
- Plugin subcommands (external dispatch via the fledge plugin system) are not represented — they live outside clap's command graph, so `introspect` only shows first-party commands
- No `--filter` flag to dump a single subcommand's subtree — today agents have to parse the full tree and navigate to the branch they want
- `ArgAction` is not exposed: `takes_value: false` currently collapses `SetTrue`, `SetFalse`, and `Count` into one bucket. Fledge doesn't use `Count` or `SetFalse` today, but if a future flag needs them, an `action` field on `ArgNode` should be added before the schema ossifies