---
module: plugin-protocol
version: 1
status: active
files:
  - src/protocol.rs
  - src/plugin.rs

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

| Export | Description |
|--------|-------------|
| `run_protocol_plugin` | Spawn a plugin in protocol mode, handling JSON-lines communication |
| `OutboundMessage` | Enum of all outbound (plugin → fledge) message types |
| `PluginContext` | Init context sent to the plugin on startup |

### Structs & Enums

| Type | Description |
|------|-------------|
| `OutboundMessage` | Enum: Prompt, Confirm, Select, MultiSelect, Progress, Log, Output, Store, Load, Exec, Metadata |
| `PluginContext` | Project info, git state, args, fledge version — sent in `init` message |
| `ExecResult` | Shell command result: exit code, stdout, stderr |
| `PluginStorage` | Key-value store backed by `state.json` |

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
  }
}
```

Fields:
- `args` — command-line arguments after the plugin name
- `project` — detected project info (null if not in a project)
- `project.git` — git info (null if not a git repo)
- `plugin` — the plugin's own metadata from plugin.toml
- `fledge` — fledge version info

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

Storage is scoped to the plugin and persisted at `~/.config/fledge/plugins/<name>/state.json`. Values must be JSON-serializable strings.

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

**Security:** Commands run in a sandboxed context:
- Working directory restricted to project root and plugin directory
- No access to fledge config directory (except the plugin's own dir)
- Inherits the user's PATH but not fledge's internal state
- Network access is allowed (plugins may need to call APIs)

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
6. Plugin-local storage is scoped to `~/.config/fledge/plugins/<name>/state.json`
7. `exec` commands are sandboxed to project root and plugin directory
8. Unknown message types are ignored in both directions (forward-compatible)
9. Malformed JSON lines are logged and skipped, not fatal
10. Fledge sends SIGTERM 5 seconds after `cancel` if plugin hasn't exited

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

## Dependencies

### Consumes
- `config` — plugin directory paths, fledge version
- `run` — project detection (language, fledge.toml)
- `plugin` — plugin resolution, manifest parsing

### Consumed By
- `plugin` — run_plugin dispatches to protocol mode when declared

## Future Considerations

These are not part of v1 but are designed to be additive:

- **Streaming output** (`output` with `stream: true`) — for long-running commands that emit output over time
- **Plugin-to-plugin calls** (`invoke` type) — let plugins call other plugins through fledge
- **UI widgets** (`table`, `tree`, `diff`) — rich terminal rendering via fledge's formatters
- **File operations** (`read_file`, `write_file`) — sandboxed file access through fledge
- **Event subscriptions** — plugins subscribe to fledge events (file changes, git operations)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-22 | Initial spec — fledge-v1 protocol with prompt, confirm, select, progress, log, output, store/load, exec, metadata |
