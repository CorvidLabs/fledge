### REQ-main-001

The implementation SHALL meet this contract: Parse CLI arguments via clap derive

### REQ-main-002

The implementation SHALL meet this contract: Dispatch each subcommand to its module's entry function

### REQ-main-003

The implementation SHALL meet this contract: Forward unknown commands to installed plugins

### REQ-main-004

The implementation SHALL meet this contract: Generate shell completions on demand

### REQ-main-005

The CLI SHALL print the top-level help and exit 0 when invoked with no subcommand.

Acceptance Criteria
- `fledge` with no arguments exits 0 and stdout contains "Usage".
- `tests/templates.rs::cli_no_args_shows_help` asserts success.

### REQ-main-006

Interactive dialoguer-based prompts SHALL not leave the terminal cursor hidden after the user interrupts with Ctrl+C.

Acceptance Criteria
- `fledge plugins search --interactive` followed by Ctrl+C exits 130 and the shell cursor remains visible.

### REQ-main-010

`main` SHALL dispatch the `spec lint` subcommand and propagate its exit status.

Acceptance Criteria
- `fledge spec lint` is reachable from the CLI and appears in `fledge introspect --json`.
- A clean run exits 0; a run with any error finding exits 1.
- Errors are written to stderr as plain text even when `--json` is active.

### REQ-main-011

The crate root SHALL declare the shared task-dependency walk module (`mod deps;`) so `fledge run`, `fledge lanes run`, and `fledge lanes validate` resolve one implementation of the walk instead of each carrying its own.

Acceptance Criteria
- `src/main.rs` declares `mod deps;`.
- `src/run.rs`, `src/lanes/execute.rs`, and `src/lanes/validate.rs` all reach `walk_task_graph` through that declaration; no module keeps a second dependency walk.
- `cargo build` resolves `crate::deps` from every one of those call sites.

