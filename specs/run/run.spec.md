---
module: run
version: 12
status: active
files:
  - src/run.rs
  - src/deps.rs

db_tables: []
depends_on: []
---

# Run

## Purpose

Task runner that reads task definitions from `fledge.toml` and executes them. Supports simple string commands, full task configs with dependencies, environment variables, and working directory overrides. When no `fledge.toml` exists, auto-detects the project type and synthesizes tasks in memory. Only suggests `fledge run --init` for unrecognized (generic) project types.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point — lists or executes tasks |
| `RunOptions` | Options: `task`, `init`, `list` |
| `detect_project_type` | Detects project ecosystem from directory contents |
| `task_defaults` | Returns default task definitions for a given project type |
| `detect_node_runner` | Detects node package manager from lock files (bun, yarn, pnpm, npm) |
| `walk_task_graph` | Shared two-set DFS for task dep graphs (used by `run`, `lanes run`, and `lanes validate`) |
| `MAX_TASK_DEPTH` | Maximum dependency nesting `walk_task_graph` descends before failing (1,000) |

### Constants

| Constant | Type | Description |
|----------|------|-------------|
| `MAX_TASK_DEPTH` | `usize` | Maximum levels of dependency *nesting* the walk descends (1,000). Beyond it `walk_task_graph` returns an error naming the bound rather than exhausting the thread stack and aborting the process. Not configurable — see invariant 20 |

### Structs & Enums

| Type | Description |
|------|-------------|
| `RunOptions` | Options: `task`, `init`, `list`, `lang`, `json`, `stream`, `args` |

### CLI Flags

| Flag | Description |
|------|-------------|
| `--init` | Create a starter `fledge.toml` |
| `-l, --list` | List available tasks |
| `--lang <LANG>` | Override the detected project type |
| `--json` | Emit a structured envelope (task list, task run, or init) |
| `--stream` | Forward the task's stdout/stderr live instead of buffering them. Opt-in; only changes `--json` runs |
| `-- <ARGS…>` | Pass-through arguments for the named task's command |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(RunOptions) -> Result<()>` | Main entry — dispatch to init/list/execute |
| `detect_project_type` | `(&Path) -> &'static str` | Detect project ecosystem (rust, node, go, python, etc.) from marker files |
| `detect_node_runner` | `(&Path) -> &'static str` | Detect Node.js package manager (npm, bun, pnpm, yarn) |
| `task_defaults` | `(&str, &Path) -> String` | Return default task TOML entries for a given project type and directory |
| `walk_task_graph` | `(&str, &impl Fn(&str) -> Option<&'graph [String]>, &mut Vec<String>, &mut HashSet<String>, &mut impl FnMut(&str) -> Result<()>) -> Result<()>` | Two-set DFS: `in_progress` is the recursion stack (a hit is a cycle, reported as an ordered walk); `completed` skips already-walked nodes so diamonds are not cycles |
| `detect_node_runner` | `(&Path) -> &'static str` | Detect node package manager (bun, yarn, pnpm, npm) from lock files in directory |

## Invariants

1. Tasks are read from `fledge.toml` in the current directory, or auto-detected from project type when no `fledge.toml` exists
2. Short-form tasks (`name = "cmd"`) and full-form (`[tasks.name]` with `cmd`, `deps`, `description`, `env`, `dir`) are both supported
3. Dependencies are executed before the task itself
4. Circular dependencies are detected with a two-set DFS (`in_progress` vs `completed`) shared via `deps::walk_task_graph` and produce an error listing the ordered cycle walk (e.g. `a → b → a`). A diamond DAG (two tasks sharing one dep) is not a cycle; the shared dep runs once
5. `--init` creates a starter `fledge.toml` if none exists
6. When auto-detecting, the task list header indicates tasks are auto-detected and suggests creating `fledge.toml` to customize
7. `--json` outputs structured JSON for both task listing and task execution
8. `--lang` overrides auto-detected project type (e.g. `rust`, `node`, `go`, `python`, `swift`, `ruby`, `java-gradle`, `java-maven`)
9. Arguments after a `--` separator are passed through to the target task's command. They apply to the named task only — dependencies always run without them
10. Pass-through is safe by construction: on POSIX the args become real shell positional parameters (`sh -c '<cmd> "$@"' fledge <args…>`), never interpolated into the command string. `"$@"` is auto-appended unless the command already references a positional (`$1`..`$9`, `$@`, `$*`, or their `${…}` forms), in which case the args fill those positionals without being doubled. With no pass-through args the invocation is identical to before the feature. On Windows (`cmd /C`) there is no `$@`; args are appended as argv (best-effort)
11. `run <task> --json` includes an `args` array in the envelope only when pass-through args were supplied; arg-less runs keep their prior envelope shape
12. Human-readable runs (no `--json`) inherit fledge's stdio: child output is already live and interleaved exactly as the child wrote it. `--json` runs buffer by default, capturing both streams for the envelope, and that default is unchanged by `--stream`'s existence
13. `--stream` is opt-in and never changes the envelope's field set. With `--stream --json` the child's bytes are mirrored **to fledge's stderr** as they arrive while still being captured, never to stdout — so stdout carries nothing but fledge's own envelopes even when the task itself prints JSON. `--stream` without `--json` is accepted and is a no-op — that path already streams
14. `--stream` honours the flag unconditionally; it does not probe for a TTY. Piped/CI runs get the same live forwarding, byte-for-byte (no colouring, prefixing, or line framing), so the behaviour is deterministic and testable
15. Under `--stream` the child inherits stdin, so a streamed task can prompt. The buffered `--json` path closes the child's stdin (`Command::output` semantics) and is therefore unusable for interactive commands
16. Ordering under `--stream --json` is guaranteed **per stream**: each of stdout and stderr is forwarded in order, and a chunk is never split by the other stream. The relative interleaving *between* the two is best-effort — they are separate OS pipes drained by separate threads. True cross-stream interleaving is only available on the inherited-terminal (human-readable) path
17. Exit-code handling is identical in both modes: the child's status is reported as `exit_code`/`success` in the envelope and a non-zero status still aborts with `Task '<name>' failed with exit code <n>`
18. `--stream` is an output mode, not a task input, so unlike pass-through args it propagates to dependency tasks
19. Failing to *mirror* never destroys the *result*: if a write to fledge's stderr fails mid-run (closed pipe, full disk), live forwarding for that stream stops, a one-line warning is attempted, and the run still completes — the envelope is printed with the child's true `exit_code`/`success` and its complete `stdout`/`stderr`. A failure to *read* the child's pipe is different and remains a hard error, because the capture would be incomplete. On Unix this requires `SIGPIPE` to be ignored for as long as mirroring (and the warning that follows it) lasts: under the default disposition `main` installs, a write to a closed pipe kills fledge outright, so none of the degradation above could run. The window ends before the envelope is written, so fledge still dies quietly when its *own* stdout is closed early
20. `--json` prints one `run_task` envelope per executed task, so a task with `deps` emits several concatenated JSON objects on stdout (one per dependency, then one for the task). This predates `--stream` and is unchanged by it: the output is a JSON *stream*, not a single document
20. The walk is depth-bounded: `deps::walk_task_graph` descends at most `MAX_TASK_DEPTH` (1,000) levels of nesting, and beyond that fails with `Dependency chain deeper than 1000 tasks (reached '<task>')`. Without the bound a long chain exhausts the thread stack and aborts the process (SIGABRT, exit 134) — a crash no caller can catch, report, or test around. The bound is on *nesting*, not task count: a wide, shallow graph of any size still walks. `fledge run`, `fledge lanes run` and `fledge lanes validate` all reach the same walker and so share the bound

## Behavioral Examples

```
# List tasks
$ fledge run
Available tasks:
  build  cargo build
  test   cargo test

# Run a task
$ fledge run build
▶️ Running task: build

# Run a task with dependencies
$ fledge run ci
▶️ Running task: lint
▶️ Running task: ci

# Init a new fledge.toml
$ fledge run --init
✓ Created fledge.toml

# List tasks as JSON
$ fledge run --json
{"schema_version": 1, "action": "run_list", "auto_detected": false, "tasks": [...]}

# Init fledge.toml as JSON
$ fledge run --init --json
{"schema_version": 1, "action": "run_init", "file": "fledge.toml", "project_type": "rust", "files_created": ["fledge.toml"]}

# Run a task with JSON output
$ fledge run test --json
{"schema_version": 1, "action": "run_task", "task": "test", "command": "cargo test", "exit_code": 0, "success": true, "stdout": "...", "stderr": "..."}

# Pass arguments through to the task command (after `--`)
$ fledge run test -- --release --quiet
▶️ Running task: test
# → runs: cargo test --release --quiet

# Pass a value through (e.g. a version)
$ fledge run set-version -- 1.2.3
# → runs: ./set-version.sh 1.2.3

# Pass-through with JSON adds an `args` array to the envelope
$ fledge run test --json -- --release
{"schema_version": 1, "action": "run_task", "task": "test", "command": "cargo test", "exit_code": 0, "success": true, "stdout": "...", "stderr": "...", "args": ["--release"]}

# Long-running task under --json: buffered by default, nothing visible until exit
$ fledge run migrate --json
{"schema_version": 1, "action": "run_task", "task": "migrate", ..., "stdout": "step 1/50…", "stderr": ""}

# Same task with --stream: progress is mirrored to stderr as it happens,
# stdout still carries only fledge's own envelope
$ fledge run migrate --json --stream 2>progress.log | jq .success
step 1/50           # ← appears live on stderr while the task runs
step 2/50
true

# One envelope per executed task — a task with deps emits several,
# concatenated (pre-existing --json behaviour, unchanged by --stream).
# Read it as a JSON stream, not a single document.
$ fledge run build --json --stream | jq -c '{task, success}'
{"task":"prep","success":true}
{"task":"build","success":true}

# Interactive task — the prompt reaches the terminal and the child keeps stdin
$ fledge run deploy --json --stream
Deploy to production? [y/N]

# --stream without --json is accepted and changes nothing (that path
# already inherits the terminal)
$ fledge run dev --stream
▶️ Running task: dev

# Override project type
$ fledge run --lang node
Available tasks:
  build  npm run build
  test   npm test
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| No fledge.toml (generic project) | File missing and project type is unrecognized | Suggest `fledge run --init` |
| No tasks defined | Empty `[tasks]` section | Error with guidance |
| Unknown task | Task name not found | List available tasks |
| Circular dependency | Task A depends on B depends on A | Error listing the ordered cycle walk (`a → b → a`) |
| Task failed | Non-zero exit code | Error with exit code (same message and code in buffered and `--stream` modes) |
| Already exists | `--init` when fledge.toml exists | Error |
| Spawn failed under `--stream` | Shell cannot be spawned, or a pipe is missing | Error contextualised as `running task '<name>'`, identical to the buffered path |
| Forwarding thread panicked | Internal failure while mirroring a stream | Error `stdout/stderr forwarding thread panicked` — the run is reported as failed rather than emitting a truncated envelope. Both forwarding threads are joined before the error escapes, so neither is left detached and still writing (the same holds when waiting on the child itself fails: the child is killed so the pumps reach EOF, then both are joined) |
| Mirror write failed under `--stream` | fledge's own stderr stops accepting writes (closed pipe, full disk, a consumer that exited) | Not an error. Live forwarding stops, a `warning: live output for task '<name>' stopped (...)` line is attempted, and the envelope is still emitted with the real exit code and full capture. On Unix a closed pipe reaches the code as `EPIPE` rather than a fatal `SIGPIPE`, so this path is reachable at all |

## Dependencies

- None (uses only std and serde/toml/console)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 9 | 2026-08-17 | Fix diamond DAGs (`a → [b, c]`, `b → d`, `c → d`) being reported as circular deps. Cycle detection now uses a shared two-set DFS (`src/deps.rs`) so a completed shared dep is skipped, not treated as a back edge. Cycle errors report the ordered walk, not a `HashSet` iteration. Genuine cycles still fail |
| 7 | 2026-08-12 | Add opt-in `--stream` to `fledge run` (#507). Human-readable runs already inherited the terminal, so the real gap was `--json`, which used `Command::output` — invisible until exit and with the child's stdin closed. `--stream --json` now tees both pipes: bytes are mirrored to fledge's **stderr** live (keeping stdout a single parseable envelope) while still being captured in full, and the child inherits stdin so it can prompt. Default buffered behaviour and every envelope field are unchanged; `--stream` without `--json` is an accepted no-op. Forwarding is unconditional (no TTY probe) and verbatim. Ordering is per-stream only; cross-stream interleaving is best-effort. New `pump`/`run_streaming` helpers with unit tests plus integration tests for mirroring, envelope purity, exit codes, deps, and the buffered default |
| 6 | 2026-06-11 | Fix `run --init` generic template emitting an unclosed quote in the commented `# lint = "echo 'add your linter'"` example (uncommenting it made fledge.toml unparseable). Pass-through examples now use flags valid when appended to `cargo test` (`--release`) instead of `--nocapture`, which cargo only accepts after its own `--` separator |
| 5 | 2026-06-07 | Add task argument pass-through: `fledge run <task> -- <args…>` forwards args to the target task's command (named task only, not deps). POSIX uses real positional params (`"$@"`, auto-appended unless the command references `$1`/`$@`/…), so values are never interpolated into the command string — no injection surface. `--json` gains an `args` array when args are supplied. Additive and backward-compatible: arg-less runs are byte-identical to before. New `references_positional`/`build_task_command` helpers with unit + injection-safety tests |
| 4 | 2026-04-26 | Doc sync, behavioral examples updated to show the post-tier-D envelope shapes for `run --json`, `run <task> --json`, and `run --init --json`. No code change |
| 3 | 2026-04-26 | Tier-D 1.0 envelope: all three `--json` paths now emit `{schema_version: 1, action, ...}`. `run --init --json` previously emitted prose ("✅ Created fledge.toml"), now `{action: "run_init", file, project_type, files_created}`, a real fix not just a wrapping. `run --list --json` adds `action: "run_list"` (was bare `{auto_detected, tasks}`). `run <task> --json` adds `action: "run_task"` (was bare `{task, command, ...}`). Three new integration tests guard each shape |
| 2 | 2026-04-23 | Add `--json` flag (list + execute), `--lang` override, `detect_node_runner` |
| 1 | 2026-04-19 | Initial spec |
| 8 | 2026-08-12 | CHG-0010-opt-in-stream-mode-forwarding-live-child-output-for-fledge-run-tasks: Opt-in --stream mode forwarding live child output for fledge run tasks |
| 11 | 2026-09-17 | bound-task-graph-walk-depth-so-a-deep-dependency-chain-fails-with-a-clear-error-instead-of-overflowing-the-stack: Bound the task-graph walk depth (`deps::MAX_TASK_DEPTH`, 1,000 levels of nesting). #513 replaced three explicit heap-stack DFS loops with the shared recursive walker, so a long dependency chain overflowed the thread stack and aborted the process (exit 134) instead of erroring — a regression in kind that #513's test plan claimed to cover but never landed. Deep chains now fail with `Dependency chain deeper than 1000 tasks`; the bound is on nesting, not task count, so a wide shallow graph is unaffected. Regression tests at the bound, one past it, at 20,000 deep, and for a wide shallow graph, plus CLI coverage for `run`, `lanes run` and `lanes validate` |
| 12 | 2026-09-18 | fix-diamond-task-dependencies-falsely-rejected-as-circular: Fix diamond task dependencies falsely rejected as circular |
