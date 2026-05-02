---
module: plugin-protocol
version: 7
status: active
files:
  - src/protocol/mod.rs
  - src/protocol/ui.rs
  - src/protocol/store.rs
  - src/protocol/exec.rs
  - src/protocol/metadata.rs
  - src/protocol/detect.rs
  - src/protocol/tests.rs

db_tables: []
depends_on:
  - plugin
  - config
  - run
---

# Plugin Protocol

## Purpose

Structured JSON-lines protocol between fledge and plugins. Gives plugins access to interactive prompts, progress reporting, project context, and persistent storage — without requiring plugins to bundle their own TUI libraries or know fledge internals. Opt-in: plugins that don't declare the protocol work exactly as before (inherited stdio, run-to-exit).

## Public API

### Exported Functions

Public API — `run_protocol_plugin`, `OutboundMessage`, and `PluginContext` are the external entry points. All other exports are crate-internal (`pub(crate)`) implementation details.

| Export | Description |
|--------|-------------|
| `run_protocol_plugin` | Spawn a plugin in protocol mode, handling JSON-lines communication |
| `OutboundMessage` | Enum: Prompt, Confirm, Select, MultiSelect, Progress, Log, Output, Store, Load, Exec, Metadata |
| `PluginContext` | Project info, git state, args, fledge version, capabilities — sent in `init` message |
| `CapabilitiesInfo` | Struct tracking whether plugin has exec, store, and metadata capabilities |
| `ProjectContext` | Struct containing project name, root path, detected language, and optional git context |
| `GitContext` | Struct describing git branch, dirty status, remote name, and sanitized remote URL |
| `PluginInfo` | Struct holding plugin name, version, and directory path |
| `FledgeInfo` | Struct containing fledge framework version |
| `InboundResponse` | Struct for sending responses back to plugins with message type, request ID, and JSON value |
| `handle_prompt` | Display an interactive text input prompt with optional default and validation |
| `handle_confirm` | Display a yes/no confirmation dialog with optional default |
| `handle_select` | Display a single-choice selection menu from a list of options |
| `handle_multi_select` | Display a multi-choice selection menu allowing multiple selections |
| `handle_progress` | Display and update a progress bar or spinner with current/total values |
| `clear_progress` | Finish and clear any active progress bar display |
| `handle_log` | Print formatted log messages with color-coded severity levels |
| `MAX_STORE_KEY_SIZE` | 256-byte limit for plugin state key sizes |
| `MAX_STORE_VALUE_SIZE` | 64 KB limit per individual plugin state value |
| `MAX_STORE_TOTAL_SIZE` | 1 MB limit for total combined plugin state size |
| `MAX_STORE_KEY_COUNT` | 256-key maximum for plugin state storage |
| `handle_store` | Persist a key-value pair to plugin's state.json with locking and validation |
| `handle_load` | Retrieve a stored value by key from plugin's state.json with shared locking |
| `MAX_EXEC_OUTPUT_SIZE` | 10 MB limit per stdout/stderr stream from executed commands |
| `handle_exec` | Execute a shell command with optional cwd and timeout, returning exit code and output |
| `wait_with_timeout` | Wait for a child process with a timeout duration |
| `kill_child` | Forcefully terminate a child process with signal handling |
| `handle_metadata` | Retrieve requested metadata (fledge config, git tags/status/log, env vars) as JSON |
| `detect_project_context` | Detect project name, root path, language, and git context from current environment |
| `sanitize_remote_url` | Strip credentials from HTTPS/HTTP git URLs |
| `detect_git_context` | Extract git branch, dirty status, remote name, and sanitized remote URL |

### Structs & Enums

| Type | Description |
|------|-------------|
| `OutboundMessage` | Enum: Prompt, Confirm, Select, MultiSelect, Progress, Log, Output, Store, Load, Exec, Metadata |
| `PluginContext` | Project info, git state, args, fledge version, capabilities — sent in `init` message |
| `CapabilitiesInfo` | Declared capabilities: exec, store, metadata (all default false) |
| `ProjectContext` | Project name, root path, detected language, and optional git context |
| `GitContext` | Git branch, dirty status, remote name, and sanitized remote URL |
| `PluginInfo` | Plugin name, version, and directory path |
| `FledgeInfo` | Fledge framework version |
| `InboundResponse` | Response message with type, request ID, and JSON value |

## Opt-In

Plugins declare protocol support in `plugin.toml`:

```toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
protocol = "fledge-v1"   # enables structured communication
```

Without `protocol`, fledge spawns the plugin with inherited stdio (current behavior). With `protocol = "fledge-v1"`, fledge captures stdin/stdout as JSON-lines pipes and sends/receives structured messages.

Stderr is never captured — plugins can always write debug output to stderr, and it goes straight to the terminal.

## Capabilities

Plugins declare what protocol features they need in a `[capabilities]` section. All capabilities default to `false`.

```toml
[capabilities]
exec = true      # can run shell commands via exec messages
store = true     # can persist/load data via store/load messages
metadata = false # can read project metadata and environment
```

**Enforcement:** When a plugin sends a message that requires a capability it hasn't declared, fledge blocks the operation:
- `exec` blocked → response with `code: 126` and error in stderr
- `store` blocked → store is silently dropped, load returns null
- `metadata` blocked → response with empty object

**Install flow:** During `fledge plugins install`, if a protocol plugin declares any capabilities, fledge displays them and asks the user to confirm. Granted capabilities are persisted in `plugins.toml` alongside the plugin entry.

**Init message:** The `init` message includes a `capabilities` object so plugins know which capabilities were granted at runtime.

## Wire Format

Each message is a single JSON object on one line, terminated by `\n`. No framing, no length prefix — just newline-delimited JSON (NDJSON).

**Direction naming:**
- **Outbound** = plugin writes to stdout (plugin → fledge)
- **Inbound** = fledge writes to plugin's stdin (fledge → plugin)

Every message has a `type` field. Outbound messages that expect a response include an `id` field (string, plugin-assigned). Fledge echoes the `id` back in the corresponding inbound response.

```
{"type": "prompt", "id": "1", "message": "Deploy target:", "default": "staging"}
{"type": "response", "id": "1", "value": "production"}
```

## Lifecycle

1. Fledge spawns the plugin binary with captured stdin/stdout
2. Fledge sends an `init` message with project context
3. Plugin runs its logic, sending outbound messages as needed
4. For each request (prompt, confirm, etc.), fledge sends back a response
5. Plugin exits with code 0 (success) or non-zero (failure)
6. If the user presses Ctrl+C, fledge sends a `cancel` message, then SIGTERM after 5s

```
fledge                          plugin
  │                               │
  │──── init ────────────────────>│
  │                               │
  │<──── progress ────────────────│
  │<──── prompt ──────────────────│
  │──── response ────────────────>│
  │                               │
  │<──── log ─────────────────────│
  │<──── output ──────────────────│
  │                               │
  │                          exit(0)
```

## Inbound Messages (fledge → plugin)

### init

Sent once, immediately after spawn. Contains project context so the plugin doesn't need to shell out for basic info.

```json
{
  "type": "init",
  "protocol": "fledge-v1",
  "args": ["staging", "--dry-run"],
  "project": {
    "name": "my-app",
    "root": "/Users/dev/my-app",
    "language": "rust",
    "git": {
      "branch": "main",
      "dirty": false,
      "remote": "origin",
      "remote_url": "https://github.com/org/my-app"
    }
  },
  "plugin": {
    "name": "fledge-deploy",
    "version": "0.1.0",
    "dir": "/Users/dev/.config/fledge/plugins/fledge-deploy"
  },
  "fledge": {
    "version": "0.9.1"
  },
  "capabilities": {
    "exec": true,
    "store": true,
    "metadata": false
  }
}
```

Fields:
- `args` — command-line arguments after the plugin name
- `project` — detected project info (null if not in a project)
- `project.git` — git info (null if not a git repo)
- `plugin` — the plugin's own metadata from plugin.toml
- `fledge` — fledge version info
- `capabilities` — which capabilities were granted (exec, store, metadata)

### response

Reply to a plugin request (prompt, confirm, select).

```json
{
  "type": "response",
  "id": "1",
  "value": "production"
}
```

- `id` — echoed from the outbound request
- `value` — the user's answer (string for prompt, bool for confirm, string for select, list of strings for multi_select)

### cancel

Sent when the user interrupts (Ctrl+C) or a timeout fires.

```json
{
  "type": "cancel",
  "reason": "user_interrupt"
}
```

Reasons: `user_interrupt`, `timeout`. The plugin should clean up and exit promptly. Fledge sends SIGTERM after 5 seconds if the plugin hasn't exited.

## Outbound Messages (plugin → fledge)

### prompt

Ask the user for text input.

```json
{
  "type": "prompt",
  "id": "1",
  "message": "Deploy target:",
  "default": "staging",
  "validate": "non_empty"
}
```

- `message` (required) — the question to display
- `default` (optional) — pre-filled value
- `validate` (optional) — built-in validator: `"non_empty"`, `"integer"`, `"path_exists"`, `"url"`

Fledge displays the prompt using its standard prompt style (dialoguer) and sends a `response` with `value` as a string.

### confirm

Ask yes/no.

```json
{
  "type": "confirm",
  "id": "2",
  "message": "Deploy to production?",
  "default": false
}
```

Response `value` is a boolean.

### select

Choose one from a list.

```json
{
  "type": "select",
  "id": "3",
  "message": "Choose environment:",
  "options": ["dev", "staging", "production"],
  "default": 0
}
```

- `options` (required) — list of choices
- `default` (optional) — index of default selection

Response `value` is the selected string.

### multi_select

Choose multiple from a list.

```json
{
  "type": "multi_select",
  "id": "4",
  "message": "Select regions:",
  "options": ["us-east-1", "eu-west-1", "ap-southeast-1"],
  "defaults": [0, 1]
}
```

- `defaults` (optional) — indices of pre-selected items

Response `value` is a list of selected strings.

### progress

Report progress. No response expected (fire-and-forget).

```json
{
  "type": "progress",
  "message": "Uploading artifacts",
  "current": 3,
  "total": 10
}
```

- `message` (required) — what's happening
- `current` / `total` (optional) — numeric progress. If omitted, fledge shows a spinner instead of a progress bar.
- Sending `{"type": "progress", "done": true}` clears the progress display.

### log

Emit a structured log message. No response expected.

```json
{
  "type": "log",
  "level": "warn",
  "message": "No deploy config found, using defaults"
}
```

- `level` — `"debug"`, `"info"`, `"warn"`, `"error"`
- `message` — the log text

Fledge formats these with its standard log styling (color-coded, prefixed with plugin name).

### output

Emit text directly to the terminal. No response expected.

```json
{
  "type": "output",
  "text": "Deployed to production in 4.2s\n"
}
```

Fledge writes `text` verbatim to stdout. This is how plugins produce their main output. Unlike `log`, this has no formatting applied.

### store

Persist a key-value pair in plugin-local storage. No response expected.

```json
{
  "type": "store",
  "key": "last_deploy_target",
  "value": "production"
}
```

Storage is scoped to the plugin and persisted at `<config_dir>/fledge/plugins/<name>/state.json` (where `<config_dir>` is the platform config directory — `~/Library/Application Support/` on macOS, `~/.config/` on Linux, `%APPDATA%\` on Windows). Values must be JSON-serializable strings.

### load

Read a value from plugin-local storage.

```json
{
  "type": "load",
  "id": "5",
  "key": "last_deploy_target"
}
```

Response `value` is the stored string, or null if not found.

### exec

Ask fledge to execute a shell command. The plugin receives the result.

```json
{
  "type": "exec",
  "id": "6",
  "command": "git tag -l 'v*' --sort=-v:refname",
  "cwd": ".",
  "timeout": 10
}
```

- `command` (required) — shell command to run
- `cwd` (optional) — working directory, relative to project root. Defaults to project root.
- `timeout` (optional) — seconds before kill, default 30

Response:
```json
{
  "type": "response",
  "id": "6",
  "value": {
    "code": 0,
    "stdout": "v0.9.1\nv0.9.0\n",
    "stderr": ""
  }
}
```

**Security:** The `cwd` parameter is validated to stay within the project root or
plugin directory, but the command string itself is unfiltered — absolute paths,
`cd /`, and arbitrary binaries all work. Exec runs as an unsandboxed child
process with the user's full permissions. Granting `exec` is equivalent to
granting the plugin full access to the system.

### metadata

Request additional project metadata beyond what `init` provides.

```json
{
  "type": "metadata",
  "id": "7",
  "keys": ["fledge_config", "git_tags", "git_status"]
}
```

Available keys:
- `fledge_config` — parsed fledge.toml for the current project
- `git_tags` — list of git tags
- `git_status` — list of changed files
- `git_log` — recent commit log (last 20)
- `env` — environment variables (filtered: no secrets)

Response `value` is an object with the requested keys.

## Error Handling

### Plugin errors

If a plugin sends malformed JSON, fledge logs a warning and ignores the line. This allows plugins to be developed incrementally — a stray `println!` won't crash the host.

### Request timeouts

If the user doesn't respond to a prompt within 5 minutes (configurable), fledge sends a `cancel` message with `reason: "timeout"`.

### Protocol mismatch

If `plugin.toml` declares `protocol = "fledge-v2"` but fledge only supports v1, fledge exits with an error suggesting a fledge upgrade.

### Unknown message types

Fledge ignores outbound messages with unknown `type` values (forward-compatible). Plugins should ignore inbound messages with unknown `type` values.

## Invariants

1. Protocol is opt-in via `protocol = "fledge-v1"` in plugin.toml
2. Without protocol declaration, plugins run with inherited stdio (no behavior change)
3. Every outbound message with an `id` field gets exactly one inbound `response` or `cancel`
4. `init` is always the first message sent to the plugin
5. Stderr is never captured — always goes to terminal
6. Plugin-local storage is scoped to `<config_dir>/fledge/plugins/<name>/state.json`
7. `exec` cwd is validated to stay within the project root or plugin directory, but the command itself is unfiltered (not a sandbox)
8. Unknown message types are ignored in both directions (forward-compatible)
9. Malformed JSON lines are logged and skipped, not fatal
10. Fledge sends SIGTERM 5 seconds after `cancel` if plugin hasn't exited
11. Capabilities default to `false` — plugins must explicitly declare what they need
12. Exec, store/load, and metadata are blocked unless the corresponding capability is granted
13. Granted capabilities are persisted in `plugins.toml` and included in the `init` message
14. Exec command stdout and stderr are each capped at 10 MB (`MAX_EXEC_OUTPUT_SIZE`). Output beyond the cap is silently truncated to prevent a plugin from exhausting host memory

## Behavioral Examples

```
# Plugin with protocol support
$ cat plugin.toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
protocol = "fledge-v1"

[[commands]]
name = "deploy"
binary = "bin/fledge-deploy"

# Running it — fledge handles all prompts natively
$ fledge deploy
? Deploy target: [staging] production
? Deploy to production? [y/N] y
  ▶ Uploading artifacts [=====>    ] 3/10
  ▶ Uploading artifacts [==========] 10/10
  ✅ Deployed to production in 4.2s

# Plugin remembers last choice
$ fledge deploy
? Deploy target: [production]
```

### Example: Minimal plugin (Python)

```python
#!/usr/bin/env python3
import sys, json

def send(msg):
    print(json.dumps(msg), flush=True)

def recv():
    return json.loads(sys.stdin.readline())

# Wait for init
init = recv()
args = init["args"]

# Ask user
send({"type": "prompt", "id": "1", "message": "Deploy target:", "default": "staging"})
resp = recv()
target = resp["value"]

# Confirm
send({"type": "confirm", "id": "2", "message": f"Deploy to {target}?"})
resp = recv()
if not resp["value"]:
    send({"type": "output", "text": "Cancelled.\n"})
    sys.exit(0)

# Do the work
send({"type": "progress", "message": "Deploying", "current": 0, "total": 3})
# ... actual deployment logic ...
send({"type": "progress", "message": "Deploying", "current": 3, "total": 3})
send({"type": "progress", "done": True})

send({"type": "output", "text": f"Deployed to {target}\n"})
```

### Example: Minimal plugin (Bash)

```bash
#!/usr/bin/env bash
send() { echo "$1"; }
recv() { read -r line; echo "$line"; }

# Wait for init
INIT=$(recv)

# Prompt
send '{"type":"prompt","id":"1","message":"Deploy target:","default":"staging"}'
RESP=$(recv)
TARGET=$(echo "$RESP" | jq -r '.value')

# Output
send "{\"type\":\"output\",\"text\":\"Deploying to $TARGET\\n\"}"
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Unsupported protocol | plugin.toml declares unknown protocol version | Error with fledge upgrade suggestion |
| Malformed JSON | Plugin writes invalid JSON to stdout | Warning logged, line skipped |
| Unknown message type | Plugin sends unrecognized type | Silently ignored (forward-compat) |
| Missing id | Request message lacks id field | Warning logged, message skipped |
| Orphaned response | Response with id that doesn't match a pending request | Warning logged, ignored |
| Exec timeout | exec command exceeds timeout | Command killed, response with non-zero code |
| Exec path escape | cwd tries to escape project/plugin directory | Error response, command not run |
| Prompt timeout | No user input for 5 minutes | Cancel sent with reason "timeout" |
| Plugin hang | Plugin doesn't exit after cancel | SIGTERM after 5s, SIGKILL after 10s |
| Exec capability denied | Plugin sends exec without `exec = true` | Response with code 126, error in stderr |
| Store capability denied | Plugin sends store/load without `store = true` | Store dropped silently, load returns null |
| Metadata capability denied | Plugin sends metadata without `metadata = true` | Response with empty object |

## Dependencies

### Consumes
- `config` — plugin directory paths, fledge version
- `run` — project detection (language, fledge.toml)
- `plugin` — plugin resolution, manifest parsing

### Consumed By
- `plugin` — run_plugin dispatches to protocol mode when declared

## Compatibility Policy

`fledge-v1` is the stable plugin contract that ships with fledge 1.0. To protect plugin authors from breakage, the following rules govern how the protocol may evolve within the `v1` major version:

1. **Additive-only.** New outbound and inbound message `type` values may be added at any time. Plugins and fledge already ignore unknown `type` values per invariant 8 (forward-compatible).
2. **No field removal.** A field present in any `v1` message — outbound or inbound — must continue to be emitted. Removing a field is a breaking change and requires a new protocol version (`fledge-v2`).
3. **No field retyping.** A field's JSON type (string, number, bool, object, array) is locked once shipped. Widening a string into an object, or a single value into an array, is a breaking change.
4. **New optional fields are allowed.** Both fledge and plugins must tolerate unknown fields on known message types — additive optional fields do not require a protocol bump.
5. **New capabilities are additive.** Adding a new entry to `[capabilities]` (e.g. `network`, `secrets`) does not break existing plugins, which simply leave the new capability undeclared and continue to work.
6. **Wire format frozen.** NDJSON over stdin/stdout, stderr never captured, `id` field for request/response correlation — these are part of the v1 contract and cannot change without a new protocol version.
7. **Init message guaranteed.** The `init` message will always be the first message sent and will always include `protocol`, `args`, `project`, `plugin`, `fledge`, and `capabilities` fields. Sub-fields within those objects are additive-only.

Any change that cannot be expressed under these rules requires a new `protocol = "fledge-v2"` declaration; `v1` plugins continue to run against `v1` semantics indefinitely.

## Future Considerations

These are not part of v1 but are designed to be additive under the policy above:

- **Streaming output** (`output` with `stream: true`) — for long-running commands that emit output over time
- **Plugin-to-plugin calls** (`invoke` type) — let plugins call other plugins through fledge
- **UI widgets** (`table`, `tree`, `diff`) — rich terminal rendering via fledge's formatters
- **File operations** (`read_file`, `write_file`) — path-validated file access through fledge
- **Event subscriptions** — plugins subscribe to fledge events (file changes, git operations)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 7 | 2026-05-02 | Remove misleading "sandboxed" language from exec security notes; clarify that cwd is validated but command string is unfiltered. Change future file_operations wording from "sandboxed" to "path-validated" |
| 6 | 2026-05-02 | Clarify public vs internal exports in single table (spec-sync requires all exports in one `Exported Functions` table). Add platform-correct storage paths using `<config_dir>` notation |
| 5 | 2026-04-29 | Fix spec-sync: consolidate all exports into standard `Exported Functions` table (custom subsection headers were not parsed by spec-sync) |
| 4 | 2026-04-29 | Document all public exports from protocol submodules (ui, store, exec, metadata, detect) after module split |
| 3 | 2026-04-27 | Security: exec command stdout/stderr capped at 10 MB each (`MAX_EXEC_OUTPUT_SIZE`) to prevent OOM from unbounded plugin output. Invariant 14 added |
| 2 | 2026-04-25 | Add Compatibility Policy, `fledge-v1` is additive-only within v1; field removal or retyping requires `fledge-v2`. Locks the 1.0 plugin contract |
| 1.1 | 2026-04-22 | Add capability manifest, `exec`, `store`, `metadata` capabilities with enforcement and install-time approval |
| 1 | 2026-04-22 | Initial spec, fledge-v1 protocol with prompt, confirm, select, progress, log, output, store/load, exec, metadata |
