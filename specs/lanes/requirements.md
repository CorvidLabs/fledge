# Lanes — Requirements

## Functional Requirements

1. Define named workflow pipelines in `fledge.toml` under `[lanes]`
2. Execute lanes as ordered sequences of steps
3. Support three step types: task references, inline commands, parallel groups
4. Validate task references before execution
5. Support `fail_fast` flag to control failure behavior
6. Support `--dry-run` to preview execution plan
7. Support `--init` to scaffold default lanes for the detected language
8. List available lanes with descriptions

## Non-Functional Requirements

1. Parallel groups must execute steps concurrently using threads
2. Lane execution must respect task dependency ordering within each step
3. `--json` flag must produce machine-parseable output for list operations
