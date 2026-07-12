---
spec: meta.spec.md
---

## User Stories

- As a fledge maintainer, I want a project's provenance (which template, ref, and fledge version created it) recorded after scaffolding so tooling can identify a project's origin
- As a fledge maintainer, I want per-file content hashes recorded so drift from the original generated content can be detected later
- As a template author, I want the template variables used at scaffold time persisted so a project's configuration is inspectable
- As a user, I don't want fledge's internal cache tracked by git after scaffolding

## Acceptance Criteria

### REQ-meta-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `write_project_meta` writes `.fledge/meta.toml` with `SourceInfo` provenance, string template variables, and a hash for each existing created file
### REQ-meta-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `compute_file_hash` returns a 64-character lowercase hex SHA-256 digest and is deterministic
### REQ-meta-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- The `.fledge` directory is created if missing
### REQ-meta-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- A `.fledge/.gitignore` is created if one does not already exist, and an existing one is never overwritten
### REQ-meta-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Created files that no longer exist on disk are silently skipped, not errored
### REQ-meta-006

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge_version` is captured from `CARGO_PKG_VERSION` at compile time and `created` is the local date as `YYYY-MM-DD`

## Constraints

- Only string-valued template variables are persisted; non-string values are dropped
- File hashes are keyed by the created file's relative path as passed in
- `.fledge/meta.toml` is TOML, serialized via `toml::to_string_pretty`

## Out of Scope

- Reading back or verifying `.fledge/meta.toml` (drift detection is a consumer's job)
- Updating an existing `meta.toml` in place (the `updated` field is always written as `None`)
- Hashing directories or non-regular files
