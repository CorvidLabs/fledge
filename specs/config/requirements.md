---
spec: config.spec.md
---

## User Stories

- As a user, I want to set default values (author, license, org) so that `fledge init` doesn't prompt me every time
- As a user, I want to manage extra template directories so I can use my own templates alongside built-ins
- As a user, I want to register remote template repos so they're discovered automatically during `fledge init`
- As a user, I want to store my GitHub token in config so remote template fetching works without env vars

## Acceptance Criteria

### REQ-config-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge config list` displays all configured values and empty list sections
### REQ-config-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge config get <key>` prints the value for any valid key (scalar or list)
### REQ-config-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge config set <key> <value>` persists scalar values to `config.toml`
### REQ-config-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge config unset <key>` removes values (clears lists for list keys)
### REQ-config-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge config add <key> <value>` appends to list keys with deduplication
### REQ-config-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge config remove <key> <value>` removes from list keys and reports whether found
### REQ-config-007

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Using `set` on a list key (or `add`/`remove` on a scalar key) produces a clear error with guidance
### REQ-config-008

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Config file is created on first write if it doesn't exist
### REQ-config-009

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Missing config file returns sensible defaults (MIT license, no author, empty lists)

## Constraints

- Config path follows platform conventions via `dirs::config_dir()`
- TOML format must remain human-editable
- Token must not appear in CLI output or error messages

## Out of Scope

- Config file encryption or secure credential storage
- Config migration between versions
- Interactive config wizard (prompts module handles interactive lanes)
