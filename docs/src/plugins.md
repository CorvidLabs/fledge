# Extend: Plugins, Config, Tools

Plugins extend fledge with community-built commands. They're external executables distributed as GitHub repos, following the git-style subcommand pattern. `fledge-<name>` becomes `fledge <name>`.

## Installing

All plugin commands live under `fledge plugins`.

```bash
# From GitHub
fledge plugins install someone/fledge-deploy

# Pin to a specific version (tag, branch, or commit)
fledge plugins install someone/fledge-deploy@v1.2.0

# Full URL works too
fledge plugins install https://github.com/someone/fledge-deploy.git

# Full URL with version pin
fledge plugins install https://github.com/someone/fledge-deploy.git@v1.2.0

# Reinstall (or upgrade a pinned plugin)
fledge plugins install someone/fledge-deploy@v2.0.0 --force
```

What happens when you install:
1. Repo gets cloned to the platform plugin directory (e.g. `~/Library/Application Support/fledge/plugins/<name>/` on macOS, `~/.config/fledge/plugins/<name>/` on Linux)
2. If `@ref` was specified, that tag/branch/commit is checked out
3. fledge reads `plugin.toml`
4. Build hook runs (or auto-detects Rust/Swift/Go/Node)
5. Command binaries get symlinked to the `plugins/bin/` directory
6. Plugin is registered in `plugins.toml` (with `pinned_ref` if pinned)

## Using Plugins

```bash
# Via fledge
fledge plugins run deploy --target production

# Or directly if the binary is on PATH
fledge deploy --target production
```

## Managing

```bash
fledge plugins list              # what's installed
fledge plugins search deploy     # find plugins on GitHub
fledge plugins update            # update all unpinned plugins
fledge plugins update fledge-deploy  # update a specific plugin
fledge plugins remove fledge-deploy
fledge plugins list --json       # for scripting
```

## Version Pinning

Use `@ref` to pin a plugin to a specific version:

```bash
fledge plugins install someone/fledge-deploy@v1.2.0
```

Pinned plugins behave differently on update:
- **Unpinned** plugins get `git pull` and rebuild automatically
- **Pinned** plugins check for newer tags and suggest an upgrade command, but don't change automatically

To upgrade a pinned plugin:
```bash
fledge plugins install someone/fledge-deploy@v2.0.0 --force
```

## Discovery

Plugins use the `fledge-plugin` topic on GitHub. To make yours findable:

1. Add `fledge-plugin` as a topic on your repo
2. Include a `plugin.toml` manifest

```bash
fledge plugins search            # browse all plugins
fledge plugins search deploy     # search by keyword
```

## Building a Plugin

### 1. Create the repo

The fastest way to start a plugin:

```bash
fledge plugins create fledge-deploy
cd fledge-deploy
```

This scaffolds `plugin.toml`, a starter executable in `bin/`, a README, and a `.gitignore`. Or create one manually:

```bash
mkdir fledge-deploy && cd fledge-deploy
```

### 2. Write plugin.toml

```toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
description = "Deploy to cloud providers"
author = "Your Name"

[capabilities]
exec = true
store = true

[[commands]]
name = "deploy"
description = "Deploy the project"
binary = "fledge-deploy"

[[commands]]
name = "rollback"
description = "Rollback to previous deployment"
binary = "fledge-rollback"

[hooks]
build = "cargo build --release"
post_install = "echo 'fledge-deploy installed'"
post_work_start = "scripts/setup-env.sh"
```

### 3. Add the executables

Each `[[commands]]` entry points to an executable in the repo. Can be compiled binaries, shell scripts, whatever:

```bash
#!/usr/bin/env bash
# fledge-deploy
echo "Deploying $(basename $(pwd))..."
```

Make them executable:

```bash
chmod +x fledge-deploy fledge-rollback
```

### 4. Validate

Check your plugin before publishing:

```bash
fledge plugins validate
```

This checks: plugin.toml is valid, name/version are set, binaries exist (or a build hook will create them), and commands are well-formed. Use `--strict` to fail on warnings, `--json` for machine-readable output.

### 5. Publish

Publish to GitHub (validates automatically before pushing):

```bash
fledge plugins publish
```

Or push manually and add the `fledge-plugin` topic. Users install with:

```bash
fledge plugins install yourname/fledge-deploy
```

## plugin.toml Reference

### [plugin]

| Field | Type | Required | |
|-------|------|----------|-|
| `name` | string | Yes | Plugin name |
| `version` | string | Yes | Semver |
| `description` | string | No | Short description (warned about if missing on `validate`) |
| `author` | string | No | Who made it |
| `protocol` | string | No | Set to `"fledge-v1"` to opt into the [structured plugin protocol](https://github.com/CorvidLabs/fledge/blob/main/specs/plugin/plugin-protocol.spec.md). Without it, the plugin runs with inherited stdio. |

### [[commands]]

Each entry registers a subcommand.

| Field | Type | Required | |
|-------|------|----------|-|
| `name` | string | Yes | Command name (`fledge plugins run <name>`) |
| `description` | string | No | What it does |
| `binary` | string | Yes | Path to executable (relative to plugin root) |

### [capabilities]

Capabilities declare what protocol features the plugin uses. All default to `false`. Plugins must opt in.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `exec` | bool | `false` | Execute shell commands on the host |
| `store` | bool | `false` | Persist key-value data between runs |
| `metadata` | bool | `false` | Read project metadata (language, name, git info) |

```toml
[capabilities]
exec = true
store = true
metadata = false
```

During installation, fledge displays the requested capabilities and the user must approve them. Granted capabilities are recorded in `plugins.toml` and enforced at runtime:

- **Blocked exec** → returns exit code 126
- **Blocked store** → silently dropped
- **Blocked metadata** → returns empty object

Plugins without a `[capabilities]` section work fine but cannot use exec, store, or metadata protocol features.

### [hooks]

Hooks fire in response to fledge lifecycle events. All fields are optional, plugins only participate in events they declare.

| Field | Type | Description |
|-------|------|-------------|
| `build` | string | Runs after clone, before binary check |
| `post_install` | string | Runs after `fledge plugins install` |
| `post_remove` | string | Runs before `fledge plugins remove` deletes files |
| `pre_init` | string | Runs before `fledge templates init` starts |
| `post_work_start` | string | Runs after `fledge work start` creates a branch |
| `pre_push` | string | Runs before `fledge work push` pushes to origin |

Values can be a path to a script (relative to plugin root) or an inline shell command.

## Using Plugins in Lanes

Plugin commands can be called from lane steps as inline commands:

```toml
[lanes.deploy]
description = "Test, build, and deploy"
steps = [
  "test",
  { run = "cargo build --release" },
  { run = "fledge deploy --target production" },
]
```

You can also run plugin commands in parallel with other tasks:

```toml
[lanes.ci]
steps = [
  { parallel = ["lint", "test"] },
  "build",
  { parallel = [{ run = "fledge deploy --target staging" }, { run = "fledge notify --channel ci" }] },
]
```

See [Run: Tasks and Lanes](./lanes.md) for full step type documentation.

## Plugin Protocol (fledge-v1)

Plugins that opt into the protocol (`protocol = "fledge-v1"` in `[plugin]`) communicate with fledge via newline-delimited JSON over stdin/stdout. When a plugin starts, fledge sends an `init` message with project context and granted capabilities, then the plugin sends outbound messages and fledge replies to anything that includes an `id`.

### Message types at a glance

Outbound (plugin → fledge):

| Type | Reply | Requires |
|------|-------|----------|
| `prompt` | `response` with string | — |
| `confirm` | `response` with boolean | — |
| `select` | `response` with selected string | — |
| `multi_select` | `response` with array of strings | — |
| `exec` | `response` with `{code, stdout, stderr}` | `exec` |
| `store` | *(fire-and-forget)* | `store` |
| `load` | `response` with string or `null` | `store` |
| `metadata` | `response` with object of requested keys | `metadata` |
| `progress` | *(fire-and-forget)* | — |
| `log` | *(fire-and-forget; level: debug/info/warn/error)* | — |
| `output` | *(fire-and-forget; printed verbatim to stdout)* | — |

Reply messages always have shape `{"type": "response", "id": "<echoed>", "value": <type-specific>}`. There is no `exec_result` / `store_ack` / `load_result` envelope — every reply uses the generic `response` type.

See the [plugin protocol spec](https://github.com/CorvidLabs/fledge/blob/main/specs/plugin/plugin-protocol.spec.md) for full schemas, lifecycle, security model, and worked examples.

## Authentication

Plugin install, update, and search operations use your GitHub token when available. This enables installing plugins from private repositories. See [Configuration: GitHub](./configuration.md#github) for the full token resolution order and required scopes.

The easiest setup is `gh auth login` — fledge uses it automatically as a fallback. The token is injected via git's `http.extraheader` mechanism and is never embedded in remote URLs or persisted to disk.

## Security Model

> **Warning:** Plugins run as unsandboxed processes with your full user permissions. A plugin can read any file you can read, write to any directory you can write to, and make network requests — regardless of its declared capabilities. Capabilities gate the fledge-v1 *protocol* (exec/store/metadata RPC messages), not the process itself. Review plugin source before installing, especially from unknown authors.

Fledge has several safeguards:

- **Install confirmation**: before cloning, fledge warns that plugins can execute arbitrary code and asks for confirmation. Pass `--force` to skip (CI/scripts).
- **Plugin name validation**: repo names are checked for path traversal (`..`, `/`, `\`, leading `.`)
- **Command name validation**: command names that become symlinks (`fledge-<name>`) are validated to reject `/`, `\`, `.` prefix, `-` prefix, and null bytes
- **Binary path traversal**: plugin binaries cannot reference paths outside the plugin directory (both sides are canonicalized to defeat symlink bypass)
- **Hook execution**: hooks run as direct processes, not via a shell. This prevents shell injection but means pipes, redirects, and shell expansions won't work in hook commands. Use a wrapper script if you need shell features.

### CI / Non-Interactive Usage

| Flag | Where | What it does |
|------|-------|-------------|
| `--force` | `fledge plugins install` | Skips the install confirmation prompt |
| `--yes` | `fledge templates init` | Skips the post-create hook confirmation prompt |

Without these flags, interactive prompts will cause CI pipelines to hang.

## File Locations

Plugin storage uses the platform config directory:

| Platform | Base path |
|----------|-----------|
| macOS    | `~/Library/Application Support/fledge/` |
| Linux    | `~/.config/fledge/` |
| Windows  | `%APPDATA%\fledge\` |

Under that base:

| Path | What's there |
|------|-------------|
| `plugins/` | Installed plugin directories |
| `plugins/bin/` | Symlinked binaries |
| `plugins.toml` | Plugin registry |
