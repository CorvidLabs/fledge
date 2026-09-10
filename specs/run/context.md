---
spec: run.spec.md
---

## Key Decisions

- Tasks are defined in `fledge.toml` under `[tasks]` — either short form (`build = "cargo build"`) or full form with deps, env, dir, and description
- Task dependencies form a DAG — circular deps are detected with a two-set DFS (`in_progress` / `completed` in `src/deps.rs`) and produce an error listing the ordered cycle walk. A diamond (two tasks sharing one dep) is a valid DAG; the shared dep runs once
- `detect_project_type()` is public because it's used by `init` (to generate default `fledge.toml`) and `doctor` (for toolchain checks)
- `task_defaults()` returns sensible default tasks per language (build, test, lint, fmt) so new projects work out of the box
- `--list` shows available tasks with descriptions — useful for discoverability
- `--init` generates a starter `fledge.toml` based on detected project type

## Files to Read First

- `src/run.rs` — task parsing, dependency resolution, execution
- `src/deps.rs` — shared two-set DFS used by `run`, `lanes run`, and `lanes validate`
- `fledge.toml` (in any project) — the task definition format
- `specs/run/run.spec.md` — formal API and invariants

## Current Status

- Fully implemented: task parsing, dep resolution, execution with env/dir support
- Auto-detection covers: rust, node, go, python, ruby, java-gradle, java-maven, swift, generic
- `task_defaults()` provides starter tasks for each detected project type

## Notes

- Tasks run via `sh -c` on Unix — command strings are shell expressions
- Dep resolution uses a two-set DFS (`walk_task_graph`) so completed nodes on another branch are skipped rather than treated as cycles
- The `generic` project type is the fallback when no language markers are found
