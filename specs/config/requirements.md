---
spec: config.spec.md
---

## User Stories

- As a user, I want to set default values (author, license, org) so that `fledge init` doesn't prompt me every time
- As a user, I want to manage extra template directories so I can use my own templates alongside built-ins
- As a user, I want to register remote template repos so they're discovered automatically during `fledge init`
- As a user, I want to store my GitHub token in config so remote template fetching works without env vars

## Durable Requirements

### REQ-config-001

The implementation SHALL satisfy the following criterion: `fledge config list` displays all configured values and empty list sections

Acceptance Criteria

- `fledge config list` displays all configured values and empty list sections

### REQ-config-002

The implementation SHALL satisfy the following criterion: `fledge config get <key>` prints the value for any valid key (scalar or list)

Acceptance Criteria

- `fledge config get <key>` prints the value for any valid key (scalar or list)

### REQ-config-003

The implementation SHALL satisfy the following criterion: `fledge config set <key> <value>` persists scalar values to `config.toml`

Acceptance Criteria

- `fledge config set <key> <value>` persists scalar values to `config.toml`

### REQ-config-004

The implementation SHALL satisfy the following criterion: `fledge config unset <key>` removes values (clears lists for list keys)

Acceptance Criteria

- `fledge config unset <key>` removes values (clears lists for list keys)

### REQ-config-005

The implementation SHALL satisfy the following criterion: `fledge config add <key> <value>` appends to list keys with deduplication

Acceptance Criteria

- `fledge config add <key> <value>` appends to list keys with deduplication

### REQ-config-006

The implementation SHALL satisfy the following criterion: `fledge config remove <key> <value>` removes from list keys and reports whether found

Acceptance Criteria

- `fledge config remove <key> <value>` removes from list keys and reports whether found

### REQ-config-007

The implementation SHALL satisfy the following criterion: Using `set` on a list key (or `add`/`remove` on a scalar key) produces a clear error with guidance

Acceptance Criteria

- Using `set` on a list key (or `add`/`remove` on a scalar key) produces a clear error with guidance

### REQ-config-008

The implementation SHALL satisfy the following criterion: Config file is created on first write if it doesn't exist

Acceptance Criteria

- Config file is created on first write if it doesn't exist

### REQ-config-009

The implementation SHALL satisfy the following criterion: Missing config file returns sensible defaults (MIT license, no author, empty lists)

Acceptance Criteria

- Missing config file returns sensible defaults (MIT license, no author, empty lists)

## Acceptance Criteria

- `fledge config list` displays all configured values and empty list sections
- `fledge config get <key>` prints the value for any valid key (scalar or list)
- `fledge config set <key> <value>` persists scalar values to `config.toml`
- `fledge config unset <key>` removes values (clears lists for list keys)
- `fledge config add <key> <value>` appends to list keys with deduplication
- `fledge config remove <key> <value>` removes from list keys and reports whether found
- Using `set` on a list key (or `add`/`remove` on a scalar key) produces a clear error with guidance
- Config file is created on first write if it doesn't exist
- Missing config file returns sensible defaults (MIT license, no author, empty lists)

## Constraints

- Config path follows platform conventions via `dirs::config_dir()`
- TOML format must remain human-editable
- Token must not appear in CLI output or error messages

## Out of Scope

- Config file encryption or secure credential storage
- Config migration between versions
- Interactive config wizard (prompts module handles interactive lanes)
