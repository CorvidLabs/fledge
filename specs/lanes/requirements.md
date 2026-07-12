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
9. Scaffold a lane repo via `fledge lanes create <name>` with example fledge.toml, README, and .gitignore
10. Validate lane definitions via `fledge lanes validate [path]` — check task references, empty steps, circular deps, parallel groups
11. `publish` validates before pushing

## Non-Functional Requirements

1. Parallel groups must execute steps concurrently using threads
2. Lane execution must respect task dependency ordering within each step
3. `--json` flag must produce machine-parseable output for list operations

## Durable Requirements

### REQ-lanes-001

The implementation SHALL satisfy the following criterion: Define named workflow pipelines in `fledge.toml` under `[lanes]`

Acceptance Criteria

- Define named workflow pipelines in `fledge.toml` under `[lanes]`

### REQ-lanes-002

The implementation SHALL satisfy the following criterion: Execute lanes as ordered sequences of steps

Acceptance Criteria

- Execute lanes as ordered sequences of steps

### REQ-lanes-003

The implementation SHALL satisfy the following criterion: Support three step types: task references, inline commands, parallel groups

Acceptance Criteria

- Support three step types: task references, inline commands, parallel groups

### REQ-lanes-004

The implementation SHALL satisfy the following criterion: Validate task references before execution

Acceptance Criteria

- Validate task references before execution

### REQ-lanes-005

The implementation SHALL satisfy the following criterion: Support `fail_fast` flag to control failure behavior

Acceptance Criteria

- Support `fail_fast` flag to control failure behavior

### REQ-lanes-006

The implementation SHALL satisfy the following criterion: Support `--dry-run` to preview execution plan

Acceptance Criteria

- Support `--dry-run` to preview execution plan

### REQ-lanes-007

The implementation SHALL satisfy the following criterion: Support `--init` to scaffold default lanes for the detected language

Acceptance Criteria

- Support `--init` to scaffold default lanes for the detected language

### REQ-lanes-008

The implementation SHALL satisfy the following criterion: List available lanes with descriptions

Acceptance Criteria

- List available lanes with descriptions

### REQ-lanes-009

The implementation SHALL satisfy the following criterion: Scaffold a lane repo via `fledge lanes create <name>` with example fledge.toml, README, and .gitignore

Acceptance Criteria

- Scaffold a lane repo via `fledge lanes create <name>` with example fledge.toml, README, and .gitignore

### REQ-lanes-010

The implementation SHALL satisfy the following criterion: Validate lane definitions via `fledge lanes validate [path]` — check task references, empty steps, circular deps, parallel groups

Acceptance Criteria

- Validate lane definitions via `fledge lanes validate [path]` — check task references, empty steps, circular deps, parallel groups

### REQ-lanes-011

The implementation SHALL satisfy the following criterion: `publish` validates before pushing

Acceptance Criteria

- `publish` validates before pushing

### REQ-lanes-012

The implementation SHALL satisfy the following criterion: Parallel groups must execute steps concurrently using threads

Acceptance Criteria

- Parallel groups must execute steps concurrently using threads

### REQ-lanes-013

The implementation SHALL satisfy the following criterion: Lane execution must respect task dependency ordering within each step

Acceptance Criteria

- Lane execution must respect task dependency ordering within each step

### REQ-lanes-014

The implementation SHALL satisfy the following criterion: `--json` flag must produce machine-parseable output for list operations

Acceptance Criteria

- `--json` flag must produce machine-parseable output for list operations
