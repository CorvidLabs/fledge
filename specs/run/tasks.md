---
spec: run.spec.md
---

## Tasks

- [x] Write run spec
- [x] Implement RunOptions struct with task, init, and list fields
- [x] Implement detect_project_type() for Rust, Node, Go, Python, Ruby, Java, and generic
- [x] Implement fledge.toml parsing with short-form and full-form task support
- [x] Implement task execution with shell dispatch
- [x] Implement task dependency resolution with execution ordering
- [x] Implement circular dependency detection
- [x] Implement --init to scaffold a starter fledge.toml from detected project type
- [x] Implement --list to display available tasks
- [x] Implement auto-detection fallback when no fledge.toml exists
- [x] Wire RunAction subcommand into main.rs
- [x] Add unit tests for detect_project_type, task parsing, circular dependency detection
- [x] Register spec and verify with cargo test, clippy, fmt, spec-check

## Gaps

- No parallel task execution
- No task output capture or streaming
- No task caching/skip-if-unchanged
