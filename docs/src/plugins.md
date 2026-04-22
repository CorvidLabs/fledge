# Plugins

Plugins extend fledge with community-built commands. They're external executables distributed as GitHub repos, following the git-style subcommand pattern. `fledge-<name>` becomes `fledge <name>`.

## Installing

```bash
# From GitHub
fledge plugin install someone/fledge-deploy

# Pin to a specific version (tag, branch, or commit)
fledge plugin install someone/fledge-deploy@v1.2.0

# Full URL works too
fledge plugin install https://github.com/someone/fledge-deploy.git

# Full URL with version pin
fledge plugin install https://github.com/someone/fledge-deploy.git@v1.2.0

# Reinstall (or upgrade a pinned plugin)
fledge plugin install someone/fledge-deploy@v2.0.0 --force
```

What happens when you install:
1. Repo gets cloned to `~/.config/fledge/plugins/<name>/`
2. If `@ref` was specified, that tag/branch/commit is checked out
3. fledge reads `plugin.toml`
4. Build hook runs (or auto-detects Rust/Swift/Go/Node)
5. Command binaries get symlinked to `~/.config/fledge/plugins/bin/`
6. Plugin is registered in `~/.config/fledge/plugins.toml` (with `pinned_ref` if pinned)

## Using Plugins

```bash
# Via fledge
fledge plugin run deploy --target production

# Or directly if the binary is on PATH
fledge deploy --target production
```

## Managing

```bash
fledge plugin list              # what's installed
fledge plugin search deploy     # find plugins on GitHub
fledge plugin update            # update all unpinned plugins
fledge plugin update fledge-deploy  # update a specific plugin
fledge plugin remove fledge-deploy
fledge plugin list --json       # for scripting
```

## Version Pinning

Use `@ref` to pin a plugin to a specific version:

```bash
fledge plugin install someone/fledge-deploy@v1.2.0
```

Pinned plugins behave differently on update:
- **Unpinned** plugins get `git pull` and rebuild automatically
- **Pinned** plugins check for newer tags and suggest an upgrade command, but don't change automatically

To upgrade a pinned plugin:
```bash
fledge plugin install someone/fledge-deploy@v2.0.0 --force
```

## Discovery

Plugins use the `fledge-plugin` topic on GitHub. To make yours findable:

1. Add `fledge-plugin` as a topic on your repo
2. Include a `plugin.toml` manifest

```bash
fledge plugin search            # browse all plugins
fledge plugin search deploy     # search by keyword
```

## Building a Plugin

### 1. Create the repo

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

### 4. Publish

Push to GitHub, add the `fledge-plugin` topic. Users install with:

```bash
fledge plugin install yourname/fledge-deploy
```

## plugin.toml Reference

### [plugin]

| Field | Type | Required | |
|-------|------|----------|-|
| `name` | string | Yes | Plugin name |
| `version` | string | Yes | Semver |
| `description` | string | No | Short description |
| `author` | string | No | Who made it |

### [[commands]]

Each entry registers a subcommand.

| Field | Type | Required | |
|-------|------|----------|-|
| `name` | string | Yes | Command name (`fledge plugin run <name>`) |
| `description` | string | No | What it does |
| `binary` | string | Yes | Path to executable (relative to plugin root) |

### [capabilities]

Capabilities declare what protocol features the plugin uses. All default to `false` — plugins must opt in.

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

Hooks fire in response to fledge lifecycle events. All fields are optional — plugins only participate in events they declare.

| Field | Type | Description |
|-------|------|-------------|
| `build` | string | Runs after clone, before binary check |
| `post_install` | string | Runs after `fledge plugin install` |
| `post_remove` | string | Runs before `fledge plugin remove` deletes files |
| `pre_init` | string | Runs before `fledge init` starts |
| `post_work_start` | string | Runs after `fledge work start` creates a branch |
| `pre_pr` | string | Runs before `fledge work pr` pushes and creates a PR |

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

See [Lanes & Pipelines](./lanes.md) for full step type documentation.

## Plugin Protocol (fledge-v1)

Plugins that declare capabilities communicate with fledge via a JSON-over-stdin/stdout protocol. When a plugin starts, fledge sends an `init` message with the plugin's granted capabilities, then the plugin sends requests and fledge responds.

### Message Types

| Plugin sends | Fledge responds with | Requires |
|-------------|---------------------|----------|
| `exec` | `exec_result` (stdout, stderr, exit code) | `exec` capability |
| `store` | `store_ack` | `store` capability |
| `load` | `load_result` (value or null) | `store` capability |
| `metadata` | `metadata_result` (project info) | `metadata` capability |
| `log` | *(no response)* | *(always allowed)* |
| `progress` | *(no response)* | *(always allowed)* |
| `output` | *(terminates plugin)* | *(always allowed)* |

See the [plugin protocol spec](https://github.com/CorvidLabs/fledge/blob/main/specs/plugin/plugin-protocol.spec.md) for full details.

## Authentication

Plugin install, update, and search operations use your GitHub token when available. This enables installing plugins from private repositories.

The token is resolved in order:
1. `FLEDGE_GITHUB_TOKEN` environment variable
2. `GITHUB_TOKEN` environment variable
3. `github.token` in `~/.config/fledge/config.toml`

```bash
# Set via config
fledge config set github.token ghp_your_token_here

# Or via environment
export GITHUB_TOKEN=ghp_your_token_here
```

The token is injected via git's `http.extraheader` mechanism — it is never embedded in remote URLs or persisted to disk.

## Security Model

Plugins run arbitrary code. Fledge has several safeguards:

- **Install confirmation** — before cloning, fledge warns that plugins can execute arbitrary code and asks for confirmation. Pass `--force` to skip (CI/scripts).
- **Plugin name validation** — repo names are checked for path traversal (`..`, `/`, `\`, leading `.`)
- **Command name validation** — command names that become symlinks (`fledge-<name>`) are validated to reject `/`, `\`, `.` prefix, `-` prefix, and null bytes
- **Binary path traversal** — plugin binaries cannot reference paths outside the plugin directory (both sides are canonicalized to defeat symlink bypass)
- **Hook execution** — hooks run as direct processes, not via a shell. This prevents shell injection but means pipes, redirects, and shell expansions won't work in hook commands. Use a wrapper script if you need shell features.

### CI / Non-Interactive Usage

| Flag | Where | What it does |
|------|-------|-------------|
| `--force` | `fledge plugin install` | Skips the install confirmation prompt |
| `--yes` | `fledge init` | Skips the post-create hook confirmation prompt |

Without these flags, interactive prompts will cause CI pipelines to hang.

## File Locations

| Path | What's there |
|------|-------------|
| `~/.config/fledge/plugins/` | Installed plugin directories |
| `~/.config/fledge/plugins/bin/` | Symlinked binaries |
| `~/.config/fledge/plugins.toml` | Plugin registry |
