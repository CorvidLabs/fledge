---
spec: meta.spec.md
---

# Meta — Tasks

- [x] Model `ProjectMeta` and `SourceInfo` provenance records
- [x] Implement `compute_file_hash` (SHA-256 hex)
- [x] Implement `write_project_meta` (`.fledge/meta.toml` + file hashes)
- [x] Ensure `.fledge` directory and `.gitignore` creation
- [x] Write meta spec and companion files

## Gaps

- No reader/verifier for `.fledge/meta.toml` — drift detection consumers are not yet implemented in this module
- `SourceInfo.updated` is always written as `None`; there is no in-place metadata refresh path
- Only string-valued template variables are recorded; nested/typed values are dropped
