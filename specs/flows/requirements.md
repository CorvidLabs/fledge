# Flows — Requirements

## Functional Requirements

1. Define named workflow pipelines in `fledge.toml` under `[flows]`
2. Execute flows as ordered sequences of steps
3. Support three step types: task references, inline commands, parallel groups
4. Validate task references before execution
5. Support `fail_fast` flag to control failure behavior
6. Support `--dry-run` to preview execution plan
7. Support `--init` to scaffold default flows for the detected language
8. List available flows with descriptions

## Non-Functional Requirements

1. Parallel groups must execute steps concurrently using threads
2. Flow execution must respect task dependency ordering within each step
3. `--json` flag must produce machine-parseable output for list operations
