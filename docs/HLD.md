# fledge: high-level design

This document explains how fledge works end to end: how a command is dispatched, how
`fledge.toml` drives tasks and lanes, how plugins are installed and run, how the ship
commands (`work`, `release`, and the GitHub plugin) fit together, and where hooks fire.
It describes the code on `main` at version 1.7.2 (`Cargo.toml`). The code and the specs
in [`specs/`](../specs) are the source of truth. When they disagree with this page, they win,
and this page should be fixed.

**Contents**

1. [Purpose](#1-purpose)
2. [Context](#2-context)
3. [Components](#3-components)
4. [Key flows](#4-key-flows)
5. [Hooks](#5-hooks)
6. [Data](#6-data)
7. [Runtime and deployment](#7-runtime-and-deployment)
8. [Security and trust boundaries](#8-security-and-trust-boundaries)
9. [Failure modes and limits](#9-failure-modes-and-limits)
10. [Decisions](#10-decisions)
11. [Glossary](#11-glossary)

## 1. Purpose

fledge is a single Rust binary that gives every repository the same small set of dev-loop
commands, whatever language is inside: scaffold (`templates`), run (`run`, `lanes`, `watch`),
spec (`spec`), AI (`ai`, `ask`, `review`), ship (`work`, `release`, `changelog`) and extend
(`plugins`, `config`, `introspect`, `completions`, `doctor`). It is used by developers and,
just as much, by AI agents: every command that matters has a `--json` envelope and a
non-interactive mode. It works with nothing configured (it detects the project type), grows
through a checked-in `fledge.toml`, and keeps anything ecosystem-specific out of core in
plugins. The product intent is written down in [`INTENT.md`](../INTENT.md) and the
[`hi/`](../hi) criteria. The agent contract is in [`AGENTS.md`](../AGENTS.md).

## 2. Context

fledge runs on the user's machine, in the user's current directory. It reads the project,
shells out to the project's own toolchain and to `git`, and reaches the network only for
GitHub (search, import, publish, clone) and for LLM providers (AI commands).

```mermaid
flowchart LR
    dev["Developer"] --> cli
    agent["AI agent<br/>--json, FLEDGE_NON_INTERACTIVE=1"] --> cli
    subgraph host["User machine"]
        cli["fledge CLI<br/>single Rust binary"]
        proj[("Project directory<br/>fledge.toml, .fledge/lanes, specs/")]
        cfg[("config_dir/fledge<br/>config.toml, plugins.toml, plugins/")]
        sh["Project toolchain<br/>via sh -c or cmd /C"]
        git["git"]
        native["Native plugins<br/>fledge-NAME executables"]
        wasm["WASM plugins<br/>Wasmtime, in-process"]
        specsync["specsync binary<br/>optional"]
        gh["gh CLI<br/>optional"]
    end
    cli --> proj
    cli --> cfg
    cli --> sh
    cli --> git
    cli --> native
    cli --> wasm
    cli --> specsync
    cli -. "token fallback" .-> gh
    native -. "fledge-plugin-github" .-> gh
    cli -->|"REST over HTTPS"| ghapi["GitHub API"]
    gh --> ghapi
    git -->|"clone, push"| ghrepos["github.com repos<br/>plugins, templates, lanes"]
    cli -->|"HTTP via corvid-ai"| llm["LLM providers<br/>Anthropic, OpenAI-compatible, Ollama"]
```

| External | Used by | How |
|---|---|---|
| Project toolchain (`cargo`, `npm`, `go`, ...) | `run`, `lanes`, `watch` | Task strings run through `sh -c` (Unix) or `cmd /C` (Windows) in the project directory ([`src/run.rs`](../src/run.rs), [`src/lanes/execute.rs`](../src/lanes/execute.rs)) |
| `git` | `work`, `release`, `changelog`, `review`, plugin and template installs | Spawned as a subprocess ([`src/work.rs`](../src/work.rs), [`src/release/git.rs`](../src/release/git.rs), [`src/plugin/install.rs`](../src/plugin/install.rs), [`src/remote.rs`](../src/remote.rs)) |
| GitHub REST API | `plugins search`, `lanes search/import`, `templates search`, the `publish` commands | `ureq` with a 30 s timeout ([`src/github.rs`](../src/github.rs)) |
| github.com git remotes | plugin install/update, remote templates, publish | `git clone` / `git push`, with the GitHub token passed as an HTTP header for github.com URLs |
| LLM providers | `ai`, `ask`, `review`, `work commit --ai`, `spec lint --ai` | Plain HTTP through the [`corvid-ai`](https://crates.io/crates/corvid-ai) crate ([`src/llm.rs`](../src/llm.rs)) |
| `specsync` | `spec check` | Delegated to when it is on `PATH`, otherwise a built-in structural check ([`src/spec/engine.rs`](../src/spec/engine.rs)) |
| `gh` | token resolution | `gh auth token` is the last fallback for a GitHub token ([`src/config.rs`](../src/config.rs)). The GitHub commands themselves live in the separate `fledge-plugin-github` repo, whose own help says it drives the `gh` CLI |

## 3. Components

### 3.1 Module map

Core is one crate. [`src/main.rs`](../src/main.rs) declares every module and holds the
top-level dispatch. [`src/cli.rs`](../src/cli.rs) holds the clap derive types. Folder modules
(`plugin/`, `lanes/`, `protocol/`, `spec/`, `release/`) split a pillar into files.

```mermaid
flowchart LR
    main["main.rs + cli.rs<br/>clap parse, dispatch"]
    subgraph runp["Run"]
        run["run.rs"]
        deps["deps.rs<br/>task graph walk"]
        lanes["lanes/"]
        watch["watch.rs"]
    end
    subgraph scaffoldp["Scaffold"]
        tinit["template_cmds.rs, init.rs"]
        tpl["templates.rs, remote.rs"]
        tauth["create_template.rs, validate.rs,<br/>publish.rs, search.rs"]
    end
    subgraph specp["Spec"]
        spec["spec/"]
    end
    subgraph aip["AI"]
        ai["ai.rs, ask.rs, review.rs"]
        llm["llm.rs"]
    end
    subgraph shipp["Ship"]
        work["work.rs"]
        release["release/"]
        changelog["changelog.rs"]
    end
    subgraph extendp["Extend"]
        plugin["plugin/"]
        protocol["protocol/"]
        config["config.rs, config_cmds.rs"]
        intro["introspect.rs, doctor.rs"]
    end
    subgraph sharedp["Shared"]
        envelope["envelope.rs"]
        trust["trust.rs"]
        github["github.rs"]
    end
    main --> runp & scaffoldp & specp & aip & shipp & extendp
    run --> deps
    lanes --> deps
    watch --> run
    watch --> lanes
    release --> lanes
    work --> plugin
    tinit --> plugin
    plugin --> protocol
    plugin --> trust
    lanes --> trust
    ai --> llm
    ai --> spec
```

| Module | Owns |
|---|---|
| [`main.rs`](../src/main.rs), [`cli.rs`](../src/cli.rs) | Parsing, the global `--non-interactive` flag, one match arm per command, the `External` catch-all that routes unknown verbs to plugins |
| [`run.rs`](../src/run.rs) | `fledge run`: reading `[tasks]`, auto-detecting a project type when there is no `fledge.toml`, building the shell command, `--json`/`--stream` output, `--init` |
| [`deps.rs`](../src/deps.rs) | `walk_task_graph`: the shared depth-first walk (cycle detection, diamond handling, a 1,000-level depth bound) used by `run` and `lanes` |
| [`lanes/`](../src/lanes) | `fledge lanes`: config loading and validation ([`mod.rs`](../src/lanes/mod.rs)), execution ([`execute.rs`](../src/lanes/execute.rs)), community search/import ([`community.rs`](../src/lanes/community.rs)), defaults, create, publish, validate |
| [`watch.rs`](../src/watch.rs) | File watching (`notify`) and re-running a task or lane |
| [`templates.rs`](../src/templates.rs), [`remote.rs`](../src/remote.rs), [`init.rs`](../src/init.rs), [`template_cmds.rs`](../src/template_cmds.rs) | Built-in templates embedded with `include_dir!`, remote templates cloned into the cache dir, Tera rendering, `post_create` hooks |
| [`create_template.rs`](../src/create_template.rs), [`validate.rs`](../src/validate.rs), [`publish.rs`](../src/publish.rs), [`search.rs`](../src/search.rs) | Template authoring, validation, publishing and GitHub search |
| [`spec/`](../src/spec) | `fledge spec`: parse, structural validation, delegation to `specsync`, `spec lint` |
| [`ai.rs`](../src/ai.rs), [`ask.rs`](../src/ask.rs), [`review.rs`](../src/review.rs), [`llm.rs`](../src/llm.rs) | Provider selection and precedence, spec-aware prompts, single and multi-model review (one thread per panel slot) |
| [`work.rs`](../src/work.rs) | Branch naming, conventional commits, guarded push, `work status` |
| [`release/`](../src/release) | Version resolution, file bumps, `CHANGELOG.md` entry, release commit, tag, push |
| [`changelog.rs`](../src/changelog.rs) | `fledge changelog`: read-only changelog from tags and commits |
| [`plugin/`](../src/plugin) | Plugin install, update, remove, list, audit, search, recommend, create, publish, validate, run, lifecycle hooks, WASM runtime ([`wasm.rs`](../src/plugin/wasm.rs)) |
| [`protocol/`](../src/protocol) | The `fledge-v1` plugin protocol: init message, message loop, capability-gated `exec`, `store`, `metadata`, and UI messages |
| [`trust.rs`](../src/trust.rs) | Trust-tier classification of a source (local, official, team, unverified) |
| [`config.rs`](../src/config.rs), [`config_cmds.rs`](../src/config_cmds.rs) | Global `config.toml`, GitHub token resolution, `fledge config` |
| [`envelope.rs`](../src/envelope.rs) | The `{schema_version, ...}` JSON envelope helpers (`resource`, `action`, `versioned`) |
| [`github.rs`](../src/github.rs) | GitHub REST GET helper, API and remote base URLs, `ensure_git_repo` |
| [`introspect.rs`](../src/introspect.rs), [`doctor.rs`](../src/doctor.rs) | Command tree as JSON (core commands only), environment diagnostics |
| [`utils.rs`](../src/utils.rs), [`prompts.rs`](../src/prompts.rs), [`spinner.rs`](../src/spinner.rs), [`versioning.rs`](../src/versioning.rs), [`meta.rs`](../src/meta.rs) | Non-interactive state, prompts, spinner, version parsing, project metadata |

### 3.2 Command dispatch

`main()` resets `SIGPIPE` to the default on Unix (so `fledge ... | head` exits quietly), then
`run()` reads `FLEDGE_NON_INTERACTIVE`, parses with clap, applies `--non-interactive` (`--ni`),
and matches on the `Commands` enum. Each arm converts the clap subcommand into the handler's
own action enum (`spec_action_from`, `lane_action_from`, `plugin_action_from`, ...) and calls it.
Any verb clap does not know lands in `Commands::External`, which is how plugin commands run.

```mermaid
flowchart TD
    argv["fledge ARGS"] --> parse["read FLEDGE_NON_INTERACTIVE,<br/>clap parses Cli, apply --ni"]
    parse -->|"bad flag or missing positional"| usage["clap usage error, exit 2"]
    parse --> match{"Commands variant"}
    match -->|"no subcommand"| help["print help, exit 0"]
    match -->|"core command"| handler["module handler<br/>run, lanes, work, release, plugins, ..."]
    match -->|"External(args)"| resolve{"plugins/bin/fledge-NAME<br/>or fledge-NAME on PATH?"}
    resolve -->|"found"| prun["plugin::run with a Run action"]
    resolve -->|"not found"| unk["unrecognized subcommand"]
    handler --> result{"Result"}
    prun --> result
    unk --> fail
    result -->|"Ok"| ok["exit 0"]
    result -->|"Err"| fail["error on stderr, exit 1"]
```

Two extra routing rules:

- `fledge lanes NAME` (no `run`) is accepted: `LaneSubcommand::External` is parsed by
  `parse_external_lane_args` into the same `Run` action, honoring `--dry-run`, `--json` and `--from`.
- Aliases: `lane` for `lanes`, `plugin` for `plugins`, `template` for `templates`.

Because plugins are dispatched as external subcommands, `fledge introspect --json` shows the
core surface only. `fledge plugins list --json` is the source for plugin verbs.

### 3.3 Output contract

Every `--json` output is built with [`envelope.rs`](../src/envelope.rs): either
`{schema_version, <resource>: [...]}` (list and query commands) or
`{schema_version, action: "<verb>", ...}` (everything else). `schema_version` is tracked per
command by constants such as `RUN_TASK_SCHEMA` and `LANES_RUN_SCHEMA`. Errors always go to
stderr as plain text, and the exit code is the contract (0 ok, 1 runtime error, 2 usage
error). Progress from hooks and plugin UI goes to stderr so a `--json` stdout stays parseable.
[`AGENTS.md`](../AGENTS.md) documents each envelope.

## 4. Key flows

### 4.1 `fledge run <task>`

`fledge.toml` is read from the **current directory only** (no upward search). Without it,
fledge detects the project type from marker files and synthesizes tasks.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Main as main.rs
    participant Run as run.rs
    participant Deps as deps.rs
    participant Shell as sh -c or cmd /C
    User->>Main: fledge run test -- --release
    Main->>Run: run(RunOptions)
    alt fledge.toml in cwd
        Run->>Run: parse [tasks], bail if empty
    else no fledge.toml
        Run->>Run: detect_project_type, auto_detect_tasks
    end
    Run->>Run: unknown task name? bail with the available list
    Run->>Deps: walk_task_graph("test")
    loop each task, dependencies first, each node once
        Deps->>Deps: cycle check on the recursion stack, depth at most 1000
        Deps->>Run: on_ready(name)
        Run->>Shell: spawn cmd in dir with env, pass-through args only for "test"
        alt --json
            Shell-->>Run: captured stdout, stderr, exit code
            Run-->>User: one run_task envelope for this task
        else human mode
            Shell-->>User: inherited stdio, live output
        end
        opt non-zero exit
            Run-->>Main: Err, the walk stops
        end
    end
    Main-->>User: exit 0, or error on stderr and exit 1
```

Details, all from [`src/run.rs`](../src/run.rs) and [`src/deps.rs`](../src/deps.rs):

- **Task forms.** `name = "cmd"` (short) or a table with `cmd`, `deps`, `env`, `dir` and
  `description`. `dir` is joined to the project directory.
- **Order.** `walk_task_graph` is a two-set DFS. `in_progress` (the recursion stack) detects a
  real cycle and reports it as an ordered walk (`a → b → a`). `completed` makes a shared
  dependency in a diamond run once. The walk is bounded at `MAX_TASK_DEPTH = 1000` so a
  pathological chain errors instead of overflowing the stack. A dependency that is not defined
  fails when the walk reaches it.
- **Pass-through args** (after `--`) apply to the named task only. On POSIX they are passed as
  real positional parameters (`sh -c '<cmd> "$@"' fledge <args>`), so they are never spliced
  into the command string. If the command already references `$1`, `$@` or `$*`, `"$@"` is not
  appended again. On Windows they are appended as separate argv entries.
- **`--json`** captures each task with `Command::output()` and prints one `run_task` envelope per
  executed task, so a task with `deps` yields a stream of JSON objects, not one document.
- **`--stream`** (with `--json`) mirrors the child's output live to **stderr** while still
  capturing it for the envelope, and lets the child keep stdin. `SIGPIPE` is ignored only while
  mirroring. A broken mirror produces a warning, not a failed run.
- **Auto-detection** (first match wins): `Cargo.toml` rust, `package.json` node (runner from the
  lockfile: bun, yarn, pnpm, else npm), `go.mod`, `pyproject.toml`/`setup.py`, `Gemfile`,
  `build.gradle(.kts)`, `pom.xml`, `Package.swift`. `fledge run --init` writes a starter
  `fledge.toml` from `task_defaults`.

### 4.2 `fledge lanes run <lane>`

A lane is an ordered list of steps in `fledge.toml`. Lanes add ordering, parallel groups,
conditions, timeouts, retries and failure policy on top of tasks.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Lanes as lanes/mod.rs
    participant Exec as lanes/execute.rs
    participant Deps as deps.rs
    participant Shell as sh -c or cmd /C
    User->>Lanes: fledge lanes run ci
    Lanes->>Lanes: load fledge.toml, merge .fledge/lanes/*.toml without overwriting
    Lanes->>Lanes: validate_lane, every task ref exists and has no dep cycle
    Lanes->>Lanes: resolve --from, a 1-based index or a step name
    alt --dry-run
        Lanes-->>User: plan, or the dry-run envelope
    else run
        Lanes->>Exec: execute_lane(lane, tasks, cwd)
        loop each step in order
            alt before --from, or its when condition is false
                Exec->>Exec: record as skipped
            else run the step, up to retries + 1 attempts
                alt task step
                    Exec->>Deps: walk deps, run each task once
                    Deps->>Shell: task cmd with env and dir
                else inline run step
                    Exec->>Shell: run string in the project dir
                else parallel step
                    Exec->>Shell: one scoped thread per item
                end
                Shell-->>Exec: exit status, or a kill at the step deadline
            end
            opt step still failing after the last attempt
                alt fail_fast (default true)
                    Exec-->>User: stop, lane failed at step N
                else fail_fast = false
                    Exec->>Exec: record the failure, continue
                end
            end
        end
        Exec-->>User: summary or lane envelope, exit 1 if any step failed
    end
```

Step semantics ([`src/lanes/mod.rs`](../src/lanes/mod.rs), [`src/lanes/execute.rs`](../src/lanes/execute.rs)):

- **Step kinds.** `"task"` (a bare string), `{ task = "x", ... }`, `{ run = "cmd", ... }`, and
  `{ parallel = ["a", { run = "cmd" }], ... }`. The full forms accept `when`, `timeout`
  (seconds), `retries` and `retry_delay` (seconds, default 1).
- **`when`** is a comma-separated list, all of which must hold: `VAR` (set and non-empty),
  `VAR=value`, `!VAR` (unset or empty) and `!VAR=value`. It reads the process environment.
- **Timeouts.** With a `timeout`, the child is spawned in its own process group (Unix) or a Job
  Object (Windows), polled every 50 ms, and the whole tree is killed at the deadline.
- **Parallel groups** use `std::thread::scope`, one thread per item. All items run to completion
  and their errors are collected into one step error.
- **Output.** In human mode task output is inherited. Under `--json` child stdout and stderr are
  discarded and only the lane envelope is printed. `release --pre-lane --json` uses a silent
  variant (`execute_lane_silent`) that prints nothing at all and always starts from step 1.
- **Imports.** `fledge lanes import owner/repo[/path][@ref]` fetches the remote `fledge.toml`
  through the GitHub contents API and writes the lanes and missing tasks to
  `.fledge/lanes/<owner>-<repo>.toml`. Existing names are never overwritten. A non-official
  source needs a confirmation, or `--yes` in non-interactive mode.
- **`fledge watch`** re-invokes `lanes::run` or `run::run` in human mode on each debounced change.

### 4.3 Plugin install

`fledge plugins install <source>` accepts `owner/repo[@ref]`, any git URL, or a local path.
Install is transactional: every failure after the plugin directory is created removes it
again, and the registry is only written at the very end.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Inst as plugin/install.rs
    participant Trust as trust.rs
    participant Git as git
    participant FS as config_dir/fledge
    User->>Inst: fledge plugins install owner/repo@v1.0.0
    Inst->>Inst: parse source, local path or git, reject . and .. segments
    Inst->>Trust: determine_trust_tier(source)
    Trust-->>Inst: local, official, team or unverified
    opt no --force or --yes, and interactive
        Inst->>User: install this plugin?
    end
    alt local path
        Inst->>FS: symlink plugins/NAME to the source dir, or copy with --copy
    else git source
        Inst->>Git: clone into plugins/NAME, depth 1 unless a ref is pinned
        opt ref pinned
            Inst->>Git: checkout ref
        end
    end
    Inst->>FS: read plugins/NAME/plugin.toml
    Inst->>Inst: unverified tier asking for exec or network? roll back and refuse
    opt capabilities or hooks declared
        Inst->>User: list them and ask to grant, auto-granted by --force
    end
    Inst->>Inst: run the build hook, or auto-build (cargo, swift, go, npm)
    opt runtime = "wasm"
        Inst->>Inst: pre-compile each module to a .cwasm cache
    end
    Inst->>FS: symlink plugins/bin/fledge-CMD for every declared command
    opt post_install hook
        Inst->>Inst: run it, a failure removes the links and the dir
    end
    Inst->>FS: write plugins.toml, temp file then rename
    Inst-->>User: installed commands, or the install envelope
```

Notes from [`src/plugin/install.rs`](../src/plugin/install.rs) and
[`src/plugin/mod.rs`](../src/plugin/mod.rs):

- `--defaults` installs `DEFAULT_PLUGINS` (currently `fledge-plugin-github@v0.4.0`,
  `fledge-plugin-deps@v0.2.0`, `fledge-plugin-metrics@v0.2.1`), continues past individual
  failures, and reports a per-plugin summary.
- Non-interactive mode implies `--force`, which both skips the confirmation and auto-grants the
  requested capabilities (with a warning on stderr).
- `link_commands` rejects command names containing `/`, `\`, NUL, or a leading `.` or `-`, and
  binaries whose path contains `..` or canonicalizes outside the plugin directory.
- The registry records the granted capabilities for protocol plugins and for any plugin with
  hooks. Lifecycle hooks only fire when `exec` is recorded there (see [Hooks](#5-hooks)).
- `fledge plugins update` does `git pull --ff-only` for an unpinned plugin, then rebuilds and
  relinks. A pinned plugin stays on its tag and the newer tag is reported, except under
  `update --defaults`, which checks out the newest tag and records it as the new pin. A new
  manifest that escalates capabilities is re-prompted and re-checked against the tier gate.
  `post_install` is not re-run on update.

### 4.4 Plugin invocation

When clap does not recognize a verb, `main.rs` asks `resolve_plugin_command(name)` for
`plugins/bin/fledge-NAME` (plus `.exe`, `.bat`, `.cmd` on Windows), then for `fledge-NAME` on
`PATH`, git-style. The manifest's `protocol` and `runtime` then pick one of three execution modes.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Main as main.rs
    participant RunP as plugin/ (mod.rs, run_plugin.rs)
    participant Proto as protocol/mod.rs
    participant Wasm as plugin/wasm.rs
    participant Proc as plugin process
    User->>Main: fledge deps --audit
    Main->>RunP: resolve_plugin_command("deps")
    RunP-->>Main: path to fledge-deps
    Main->>RunP: run_plugin_cmd("deps", args)
    RunP->>RunP: registry entry and plugin.toml give protocol, runtime, capabilities
    alt no protocol declared
        RunP->>Proc: exec binary with args and FLEDGE_PLUGIN_DIR, inherited stdio
        Proc-->>User: output directly
    else protocol fledge-v1, native
        RunP->>Proto: run_protocol_plugin
        Proto->>Proc: spawn with piped stdin and stdout, stderr inherited
        Proto->>Proc: init message with args, project, plugin, fledge version, capabilities
        loop one JSON message per stdout line until EOF
            Proc->>Proto: prompt, confirm, select, progress, log, output, store, load, exec, metadata
            Proto->>Proto: check the granted capability for store, load, exec, metadata
            Proto-->>Proc: response line on stdin, or a denial
        end
    else protocol fledge-v1, runtime wasm
        RunP->>Wasm: run_wasm_plugin
        Wasm->>Wasm: load cached module, WASI preopens from the filesystem capability
        Wasm->>Wasm: run with fuel limit, 60 s epoch deadline, 256 MB memory cap
    end
    RunP-->>Main: Ok, or Err with the plugin exit code
    Main-->>User: exit status
```

The `fledge-v1` protocol ([`src/protocol/`](../src/protocol),
[`specs/plugin/plugin-protocol.spec.md`](../specs/plugin/plugin-protocol.spec.md)):

- **Transport.** Newline-delimited JSON. fledge writes an `init` message, then answers the
  plugin's requests with `{type: "response", id, value}`. Malformed lines are skipped with a
  warning. The run ends when the plugin closes stdout and exits.
- **UI messages** (`prompt`, `confirm`, `select`, `multi_select`, `progress`, `log`, `output`)
  are rendered by fledge, so plugins get the same prompts and progress bars as core.
- **Capability-gated messages.** `exec` runs `sh -c` with a `cwd` confined to the project or
  plugin directory, a 30 s default and 300 s maximum timeout, and 10 MB per output stream.
  `store`/`load` persist to `plugins/NAME/state.json` under a file lock (64 KB per value, 1 MB
  total, 256 keys). `metadata` answers keys such as `fledge_config`, `git_tags` and `git_status`.
  Denied requests get a well-formed denial (for `exec`, exit code 126) instead of hanging.
- **The init message** carries the project name, root, language and git context. Credentials
  embedded in an `https://user:token@...` remote URL are stripped first
  ([`src/protocol/detect.rs`](../src/protocol/detect.rs)).
- **WASM plugins** ([`src/plugin/wasm.rs`](../src/plugin/wasm.rs)) get the same init message and
  message handling through host functions in the `fledge` import module (`send`, `recv`, `exit`,
  and `exec`, `store_set`, `store_get`, `metadata` linked only when granted). `filesystem =
  "project"` preopens the project read-only at `/project`. `"plugin"` adds a read-write
  `/plugin` data dir. `network = true` inherits the host network. Interactive UI messages are
  not supported in WASM. The runtime is a default cargo feature (`wasm`).

### 4.5 Ship: `work`, GitHub, and `release`

`fledge work` wraps the branch, commit and push loop with guardrails. PR, issue and CI
commands are **not in core**: since the v0.15 tight-core refactor they live in
`fledge-plugin-github`, reached as `fledge github ...` through plugin dispatch (4.4).
`fledge work pr` now prints a pointer to `fledge github prs create` and exits 1.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Work as work.rs
    participant Hooks as plugin lifecycle hooks
    participant Git as git
    participant GH as fledge github plugin
    User->>Work: fledge work start add-search --issue 42
    Work->>Git: status --porcelain, must be clean
    Work->>Work: branch from branch_format, e.g. author/feat/42-add-search
    Work->>Git: checkout -b BRANCH BASE
    Work->>Hooks: post_work_start, errors ignored
    User->>Work: fledge work commit -m "add search index"
    Work->>Git: add -A only with --all, then commit with a conventional prefix
    User->>Work: fledge work push
    Work->>Work: refuse the default branch, refuse 0 commits ahead
    Work->>Hooks: pre_push, an error aborts the push
    Work->>Git: push -u origin BRANCH, --force-with-lease only with --force
    User->>GH: fledge github prs create --fill
    GH-->>User: PR number and URL
```

- **Branch names** come from `[work].branch_format` (default `{author}/{type}/{name}`), with
  `{issue}` available and `--issue N` prefixing the name as `N-name`. `{author}` comes from
  `defaults.author` in the global config or git `user.name`. Types are checked against
  `[work].branch_types` or the built-in list, unless `--prefix` is used.
- **Commits.** The type is `--type`, else inferred from the branch prefix, else
  `[work].default_type`. A message that already has a conventional prefix (including `Add:`,
  `Update:`, `Remove:`) is kept as is. `--ai` drafts the message from the staged diff.
- **`work status --json`** (schema v2) reports branch, default branch, ahead, behind and dirty count.

`fledge release <bump>` cuts a release locally. It does not publish binaries: that happens in CI
when the tag lands (section 7).

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Rel as release/mod.rs
    participant Lanes as lanes
    participant Files as version files
    participant Git as git
    User->>Rel: fledge release minor --pre-lane ci --push
    Rel->>Git: preflight, .git exists and status is clean unless --allow-dirty
    opt --pre-lane
        Rel->>Lanes: run the lane, silently under --json
    end
    Rel->>Rel: target version, explicit or a bump of plugin.toml, the manifest, or the latest tag
    alt --dry-run
        Rel-->>User: plan or dry-run envelope, nothing written
    else release
        Rel->>Files: bump Cargo.toml, package.json, pyproject.toml and [release].files
        Rel->>Git: log PREV_TAG..HEAD, prepend a CHANGELOG.md entry
        Rel->>Git: add bumped files and CHANGELOG.md, commit "chore: release vX.Y.Z"
        opt unless --no-tag
            Rel->>Git: tag -a vX.Y.Z
        end
        opt --push
            Rel->>Git: push, then push origin vX.Y.Z
        end
        Rel-->>User: summary or release envelope
    end
```

`--no-bump` makes a tag-only release and `--no-changelog` skips the changelog. Files listed in
`[release].files` must stay inside the project directory. In each one, the first match of
`version = "X.Y.Z"` (or `version: X.Y.Z`) is rewritten. This repo lists `flake.nix` there.
Gitignored files are filtered out before `git add`.

## 5. Hooks

fledge has two hook families. **Plugin hooks** are declared in a plugin's `plugin.toml`
`[hooks]` table. **Template hooks** (`post_create`) are declared in a template's
`template.toml`. Plugin lifecycle hooks are global: they fire for every installed plugin that
declares the event, whichever project you are in.

```mermaid
flowchart LR
    subgraph ph["plugin.toml [hooks]"]
        build["build"]
        post_install["post_install"]
        post_remove["post_remove"]
        pre_init["pre_init"]
        post_work_start["post_work_start"]
        pre_push["pre_push"]
    end
    subgraph th["template.toml [hooks]"]
        post_create["post_create"]
    end
    install["plugins install"] --> build
    install --> post_install
    update["plugins update"] --> build
    remove["plugins remove"] --> post_remove
    tinit["templates init"] --> pre_init
    tinit --> post_create
    wstart["work start"] --> post_work_start
    wpush["work push"] --> pre_push
```

| Hook | Fires | On failure | Gate |
|---|---|---|---|
| `build` | install and update, in place of the auto-detected build | install rolls back, update fails | shown at install for approval |
| `post_install` | after commands are linked, before the registry is written | links and plugin dir are removed | shown at install for approval |
| `post_remove` | during `plugins remove`, before the plugin dir is deleted | removal stops (see section 9) | none beyond the original install |
| `pre_init` | start of `templates init` | ignored | registry entry must have `exec` granted |
| `post_work_start` | after `work start` creates the branch | ignored | registry entry must have `exec` granted |
| `pre_push` | before `work push` runs `git push` | the push is aborted | registry entry must have `exec` granted |
| `post_create` (template) | after a template is rendered, each command through `sh -c` in the new project | `templates init` fails and shows the hook's output | local templates: `--yes` or non-interactive auto-confirms. Remote templates: only `--trust-hooks` or `FLEDGE_TRUST_HOOKS=1`, otherwise skipped when non-interactive |

How a plugin hook runs ([`src/plugin/run_plugin.rs`](../src/plugin/run_plugin.rs), `run_hook`):
if the hook value names a file inside the plugin directory it is executed (after checking it
does not escape the directory), otherwise it is split with `shell_words` and executed directly,
not through a shell. The working directory is the plugin directory, and the environment adds
`FLEDGE_PLUGIN_DIR` and `FLEDGE_REPO_ROOT` (the git toplevel of the invoking repo, falling back
to the current directory). Hook progress is written to stderr.

## 6. Data

fledge has no database. Its state is a handful of TOML and JSON files.

### 6.1 `fledge.toml` (per project)

One file configures tasks, lanes, `work` and `release`. Each module deserializes only the tables
it needs (`run.rs`, `lanes/mod.rs`, `work.rs`, `release/bump.rs`), so unknown tables are ignored.
Imported lane files in `.fledge/lanes/*.toml` have the same shape.

```mermaid
erDiagram
    FLEDGE_TOML ||--o{ TASK : "tasks table"
    FLEDGE_TOML ||--o{ LANE : "lanes table"
    FLEDGE_TOML ||--o| WORK : "work table"
    FLEDGE_TOML ||--o| RELEASE : "release table"
    LANE ||--|{ STEP : "steps"
    STEP }o--o{ TASK : "references"
    TASK }o--o{ TASK : "deps"
    IMPORTED_LANES ||--o{ LANE : "adds, never overwrites"
    IMPORTED_LANES ||--o{ TASK : "adds, never overwrites"
    TASK {
        string cmd "required, or the whole value in short form"
        string_list deps
        string_map env
        string dir "relative to the project dir"
        string description
    }
    LANE {
        string description
        bool fail_fast "default true"
    }
    STEP {
        string kind "task, run or parallel"
        string when "env condition"
        int timeout "seconds"
        int retries
        int retry_delay "seconds, default 1"
    }
    WORK {
        string branch_format
        string default_type "default feat"
        string_list branch_types
    }
    RELEASE {
        string_list files "extra version files"
    }
```

The full reference, with examples, is on the docs site under *fledge.toml* and in
[`specs/run/run.spec.md`](../specs/run/run.spec.md) and
[`specs/lanes/lanes.spec.md`](../specs/lanes/lanes.spec.md).

### 6.2 Plugin manifest and registry

```mermaid
classDiagram
    direction LR
    class PluginManifest {
        PluginMeta plugin
        List~PluginCommand~ commands
        PluginHooks hooks
        PluginCapabilities capabilities
    }
    class PluginMeta {
        String name
        String version
        String description
        String author
        String protocol
        String runtime
    }
    class PluginCommand {
        String name
        String binary
        String description
    }
    class PluginHooks {
        String build
        String post_install
        String post_remove
        String pre_init
        String post_work_start
        String pre_push
    }
    class PluginCapabilities {
        bool exec
        bool store
        bool metadata
        String filesystem
        bool network
    }
    class PluginsRegistry {
        List~PluginEntry~ plugins
    }
    class PluginEntry {
        String name
        String source
        String version
        String installed
        List~String~ commands
        String pinned_ref
        PluginCapabilities capabilities
        String runtime
    }
    PluginManifest --> PluginMeta
    PluginManifest --> PluginCommand
    PluginManifest --> PluginHooks
    PluginManifest --> PluginCapabilities
    PluginsRegistry --> PluginEntry
    PluginEntry ..> PluginCapabilities : granted copy
```

`plugin.toml` lives in each plugin's directory and is re-read on every invocation.
`plugins.toml` is the registry of what is installed and what was granted. `protocol` is
`"fledge-v1"` or absent. `runtime` is `"wasm"` or absent (`"native"` is accepted by validation).
`filesystem` is `"none"`, `"project"` or `"plugin"`.

### 6.3 File locations

`config_dir` is the platform config directory from the `dirs` crate: `~/Library/Application
Support` on macOS, `~/.config` on Linux, `%APPDATA%` on Windows.

| Path | Contents | Written by |
|---|---|---|
| `./fledge.toml` | tasks, lanes, work, release | the user, `run --init`, `lanes init` |
| `./.fledge/lanes/<owner>-<repo>.toml` | imported lanes and tasks, headed `# Imported from ...` | `lanes import` |
| `config_dir/fledge/config.toml` | global config (author, GitHub token and org, AI providers, trust orgs and users, template repos). `FLEDGE_CONFIG_DIR` overrides the directory for this file only | `fledge config`, `ai use` |
| `config_dir/fledge/plugins.toml` | plugin registry | install, update, remove (atomic rename) |
| `config_dir/fledge/plugins/NAME/` | plugin checkout or symlink, `plugin.toml`, protocol `state.json`, WASM `data/`, `.cwasm` caches | install, update, the plugin itself |
| `config_dir/fledge/plugins/bin/fledge-CMD` | symlinks to plugin command binaries (a copy on Windows if symlinks are not allowed) | install, update |
| `cache_dir/fledge/templates/<owner>/<repo>` | cached remote template clones | `templates init owner/repo` |

## 7. Runtime and deployment

fledge is a local CLI. Nothing runs as a service. Distribution is binaries and crates.

```mermaid
flowchart LR
    rel["fledge release X --push<br/>pushes tag vX.Y.Z"] --> test
    subgraph wf["release.yml, on v* tags"]
        test["test gate<br/>fmt, clippy, test"] --> build["build matrix<br/>Linux musl x86_64 and aarch64,<br/>macOS x86_64 and aarch64, Windows x86_64"]
        build --> ghrel["GitHub Release<br/>binaries and .sha256 sidecars"]
        ghrel --> pub["cargo publish,<br/>then read crates.io back"]
    end
    ghrel --> brew["post-release-formula.yml<br/>updates CorvidLabs/homebrew-tap"]
    ghrel --> dl["install.sh and the<br/>CorvidLabs/fledge action"]
    pub --> crates["cargo install fledge"]
```

- **CI** ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)) runs on every PR and push to
  `main` with no path filter, because its jobs are required checks: `cargo test` on Linux, macOS
  and Windows, fmt and clippy, `cargo audit`, an integration job that builds fledge and runs
  `fledge lanes run check` and the default plugins, the SpecSync action plus
  `specsync change check`, and `hi check`.
- **Trust gate** ([`.github/workflows/trust.yml`](../.github/workflows/trust.yml)) runs the
  CorvidLabs Trust action on PRs and `main`. Locally the same gate is `fledge trust verify`,
  whose lifecycle command is `fledge lanes run verify-native` ([`.trust.toml`](../.trust.toml)).
- **Release** ([`.github/workflows/release.yml`](../.github/workflows/release.yml)) is described
  above. The publish job needs the `CARGO_REGISTRY_TOKEN` repository secret and fails loudly
  without it. Release steps for maintainers are in [`CONTRIBUTING.md`](../CONTRIBUTING.md).
- **GitHub Action** ([`action.yml`](../action.yml)) installs a released binary in CI and verifies
  it against the `.sha256` sidecar.
- **GitHub Pages** ([`.github/workflows/pages.yml`](../.github/workflows/pages.yml)) builds the
  Astro project in [`site/`](../site). That site is now **redirect-only**: every route redirects
  to the CorvidLabs hub, where the user docs live
  ([`site/src/data/hub.ts`](../site/src/data/hub.ts),
  [`site/src/components/Redirect.astro`](../site/src/components/Redirect.astro)). The Pages
  build still publishes the atlas spec-coverage badges used by the README. This HLD is not
  published there; read it here on GitHub, which renders the Mermaid diagrams.
- **Build features.** `wasm` (Wasmtime and WASI) is on by default. `--no-default-features` builds
  without it, and WASM plugins then fail with a clear error.

## 8. Security and trust boundaries

The user's own project and config are trusted. Everything fetched from elsewhere is classified
by trust tier before it can run, and the prompts and gates scale with the tier.
[`SECURITY.md`](../SECURITY.md) is the full policy.

```mermaid
flowchart TD
    src["source string"] --> isLocal{"local path?<br/>./ ../ / . .. or local:"}
    isLocal -->|"yes"| local["local"]
    isLocal -->|"no"| norm["strip github.com URL prefix and .git"]
    norm --> trav{"a . or .. path segment?"}
    trav -->|"yes"| unv["unverified"]
    trav -->|"no"| owner{"owner"}
    owner -->|"CorvidLabs"| off["official"]
    owner -->|"built-in team list,<br/>or trust.orgs / trust.users"| team["team"]
    owner -->|"anyone else"| unv
    unv --> refuse["plugin asking for exec or network:<br/>install refused"]
    unv --> warn["lanes, hooks: warning and confirmation"]
    off --> allowed["install proceeds,<br/>capabilities still shown"]
    team --> allowed
    local --> allowed
```

- **Trust tiers** ([`src/trust.rs`](../src/trust.rs)) apply to plugins, imported lanes and
  search results. `fledge config add trust.orgs <owner>` promotes an owner to `team`. Install
  rejects `.` and `..` segments outright so a source like `CorvidLabs/../x/y` cannot borrow the
  official tier (git would collapse the path while classification keyed on the first segment).
- **Native plugins are not sandboxed.** A native plugin is arbitrary code running as the user.
  Capabilities gate only the `fledge-v1` protocol requests (`exec`, `store`, `metadata`), not
  what the process itself can do. Granting `exec` means unrestricted shell access.
- **WASM plugins are sandboxed** by Wasmtime: no filesystem, network or exec unless granted,
  fuel and a 60 s wall clock bound compute, and linear memory is capped at 256 MB. Compiled
  `.cwasm` caches are validated against the SHA-256 of the `.wasm` and the Wasmtime version.
- **Hooks and tasks run shell code.** Lane imports, remote template `post_create` hooks and plugin
  hooks all need explicit consent. Non-interactive mode never silently authorizes remote template
  hooks (that needs `--trust-hooks`). It does auto-confirm plugin installs and auto-grant their
  capabilities, so agents should install only sources they would install by hand. The
  unverified-tier refusal of `exec` and `network` still applies with `--force`.
- **Secrets.** The GitHub token is resolved from `FLEDGE_GITHUB_TOKEN`, then `GITHUB_TOKEN`, then
  `github.token` in `config.toml`, then `gh auth token`. It is sent as an `Authorization` header
  (never embedded in a URL) for GitHub API calls and github.com clones. AI keys come from
  `<PROVIDER>_API_KEY` or `ai.<provider>.api_key`. `fledge config` knows which keys are secret
  (`Config::is_secret_key`).
- **Supply chain.** Release binaries ship with `.sha256` sidecars. The GitHub Action verifies
  them. `install.sh` downloads the release binary over HTTPS but does not check the sidecar.
  `cargo audit` runs in CI.

## 9. Failure modes and limits

| Situation | Behavior |
|---|---|
| No `fledge.toml` and an unrecognized project | `run` fails with a hint to run `fledge run --init`. `lanes` always needs a `fledge.toml` |
| `fledge.toml` has `[tasks]` but it is empty | `run` fails with "No tasks defined" |
| Swift project with no `fledge.toml` | detected as `swift`, but `auto_detect_tasks` has no Swift entry, so no tasks are listed. `run --init` still writes Swift defaults |
| Task dependency cycle | error naming the ordered cycle. `lanes run` checks every step before starting. `run` finds it during the walk |
| Dependency chain deeper than 1,000 | error instead of a stack overflow (`MAX_TASK_DEPTH`) |
| Task or step exits non-zero | `run` stops at that task. A lane stops (`fail_fast = true`) or records it and continues. Exit 1 either way |
| Step `timeout` reached | the whole process tree is killed, the step fails with "step timed out", retries apply |
| Lane under `--json` | child output is discarded, so diagnose with a human-mode run |
| `plugins.toml` unreadable or mid-write | one 50 ms retry, then a warning. Plugin verbs resolve as unrecognized for that invocation |
| Plugin install fails at any step | the plugin dir (and links, after linking) are removed. The registry is untouched |
| Failing `post_remove` hook | `plugins remove` returns the error after the command links were already deleted, leaving the plugin dir and registry entry in place |
| Plugin requires an unknown `protocol` | refused with "update fledge" |
| Protocol plugin sends malformed JSON | the line is skipped with a warning |
| Plugin `exec` request | 30 s default, 300 s maximum, 10 MB per stream |
| Plugin state store | 256-byte keys, 64 KB values, 1 MB total, 256 keys |
| WASM plugin runs too long or too big | fuel exhaustion, a 60 s epoch deadline, or the 256 MB memory limit terminates it |
| GitHub API | 30 s timeout per request. Status errors (for example rate limits) become readable messages |
| LLM call | timeout from `FLEDGE_AI_TIMEOUT` or provider config. In a multi-model review a failing slot reports its own `error` and the others still return |
| Prompt needed in non-interactive mode | confirmations behave as `--yes`. Prompts without a default fail fast with the flag to pass |
| Broken pipe on stdout | fledge exits quietly (default `SIGPIPE`), except while `--stream` is mirroring |

## 10. Decisions

The design decisions live next to each module's spec as `specs/<module>/context.md`. The ones
that shape the architecture:

- **Tight core, plugins for the rest.** GitHub, dependency audits and metrics moved out of core
  in v0.15 and came back as the default plugins (`DEFAULT_PLUGINS` in
  [`src/plugin/mod.rs`](../src/plugin/mod.rs)).
- **Git-style plugin commands.** `fledge NAME` resolves to `fledge-NAME`, with symlinks in
  `plugins/bin/` so PATH is never modified ([`specs/plugin/context.md`](../specs/plugin/context.md)).
- **One file for tasks and lanes.** Lanes share `fledge.toml`, parallel groups use threads rather
  than async, and imports never overwrite ([`specs/lanes/context.md`](../specs/lanes/context.md)).
- **One graph walk.** `run`, `lanes run` and `lanes validate` share `walk_task_graph`, so they
  cannot disagree about diamonds, cycles or depth ([`specs/run/context.md`](../specs/run/context.md)).
- **stdout belongs to envelopes.** `--stream` mirrors to stderr and hook progress goes to stderr,
  so `--json` output stays machine-readable ([`specs/run/context.md`](../specs/run/context.md)).
- **Sandbox by default for WASM.** Capabilities are opt-in and prompted
  ([`specs/plugin/context-wasm.md`](../specs/plugin/context-wasm.md),
  [`specs/plugin/plugin-wasm.spec.md`](../specs/plugin/plugin-wasm.spec.md)).
- **AI over plain HTTP.** Providers moved from a CLI to HTTP in 1.5.0 ([`MIGRATION.md`](../MIGRATION.md)).
- **Specs are the contract.** SpecSync governs `src/` and `templates/`
  ([`.specsync/config.toml`](../.specsync/config.toml)), and the change history is in
  [`CHANGELOG.md`](../CHANGELOG.md).

## 11. Glossary

| Term | Meaning |
|---|---|
| Task | A named shell command in `[tasks]`, optionally with deps, env and dir |
| Lane | A named, ordered pipeline of steps in `[lanes.NAME]` |
| Step | One entry in a lane: a task ref, an inline `run`, or a `parallel` group |
| Envelope | The `{schema_version, ...}` JSON object a `--json` command prints |
| Native plugin | An executable plugin, run as the user, unsandboxed |
| Protocol plugin | A plugin with `protocol = "fledge-v1"`, which talks JSON lines with fledge |
| WASM plugin | A protocol plugin with `runtime = "wasm"`, run inside Wasmtime |
| Capability | A permission a plugin requests (`exec`, `store`, `metadata`, `filesystem`, `network`) and the user grants at install |
| Trust tier | `local`, `official`, `team` or `unverified`, derived from a source's path or owner |
| Lifecycle hook | A plugin hook fired by a core command (`pre_init`, `post_work_start`, `pre_push`) |
| Spec | A `specs/<module>/*.spec.md` contract, validated by SpecSync |
| hi | Human Intent: the plain-language criteria in [`hi/`](../hi) |
