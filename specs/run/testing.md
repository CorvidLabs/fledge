---
spec: run.spec.md
---

## Test Plan

### Unit Tests

- `detect_project_type` correctly identifies rust, node, go, python, ruby, java-gradle, java-maven, swift, and generic projects
- `task_defaults` returns non-empty task maps for each supported project type
- Circular dependency detection catches direct cycles (A→B→A) and indirect cycles (A→B→C→A)
- Short-form task (`"cargo build"`) and full-form task (with deps, env, dir) both parse correctly

### Integration Tests

- `fledge run build` in a Rust project executes `cargo build`
- `fledge run --list` displays task names and descriptions from `fledge.toml`
- `fledge run --init` in a Rust project generates a valid `fledge.toml` with rust-specific tasks
- `fledge run nonexistent` fails with available task names listed
- Task with dependencies runs deps first in correct order
