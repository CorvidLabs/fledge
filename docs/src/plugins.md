# Plugins

Extend fledge with community plugins. Plugins are external executables distributed as GitHub repositories, following the git-style subcommand pattern (`fledge-<name>` becomes `fledge <name>`).

## Installing Plugins

```bash
# From GitHub (owner/repo shorthand)
fledge plugin install someone/fledge-deploy

# From a full URL
fledge plugin install https://github.com/someone/fledge-deploy.git

# Reinstall an existing plugin
fledge plugin install someone/fledge-deploy --force
```

When you install a plugin, fledge:
1. Clones the repository to `~/.config/fledge/plugins/<name>/`
2. Reads the `plugin.toml` manifest
3. Symlinks command binaries to `~/.config/fledge/plugins/bin/`
4. Registers the plugin in `~/.config/fledge/plugins.toml`

## Using Plugins

Once installed, run plugin commands directly:

```bash
# Via fledge plugin run
fledge plugin run deploy --target production

# Or as a subcommand (if the binary is on PATH)
fledge deploy --target production
```

## Managing Plugins

```bash
# List installed plugins
fledge plugin list

# Search for plugins on GitHub
fledge plugin search deploy

# Remove a plugin
fledge plugin remove fledge-deploy

# JSON output for scripting
fledge plugin list --json
fledge plugin search --json
```

## Plugin Discovery

Plugins are discovered on GitHub using the `fledge-plugin` topic. To make your plugin discoverable:

1. Add the `fledge-plugin` topic to your GitHub repository
2. Include a valid `plugin.toml` manifest

Search for available plugins:

```bash
fledge plugin search            # browse all plugins
fledge plugin search deploy     # search by keyword
```

## Creating a Plugin

### 1. Create the Repository

```bash
mkdir fledge-deploy && cd fledge-deploy
```

### 2. Write the Manifest

Create `plugin.toml` in the repository root:

```toml
[plugin]
name = "fledge-deploy"
version = "0.1.0"
description = "Deploy to cloud providers"
author = "Your Name"

[[commands]]
name = "deploy"
description = "Deploy the project"
binary = "fledge-deploy"

[[commands]]
name = "rollback"
description = "Rollback to previous deployment"
binary = "fledge-rollback"

[[hooks]]
event = "lane:post"
binary = "fledge-deploy-notify"
```

### 3. Add the Command Binaries

Each `[[commands]]` entry points to an executable file in the repository. These can be compiled binaries, shell scripts, or any executable:

```bash
#!/usr/bin/env bash
# fledge-deploy
echo "Deploying $(basename $(pwd))..."
```

Make sure binaries are executable:

```bash
chmod +x fledge-deploy fledge-rollback
```

### 4. Publish

Push to GitHub and add the `fledge-plugin` topic to the repository. Users can then install with:

```bash
fledge plugin install yourname/fledge-deploy
```

## plugin.toml Reference

### [plugin] Section

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Plugin name |
| `version` | string | Yes | Semantic version |
| `description` | string | No | Short description |
| `author` | string | No | Plugin author |

### [[commands]] Entries

Each entry registers a subcommand that fledge can run.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Command name (becomes `fledge plugin run <name>`) |
| `description` | string | No | Command description |
| `binary` | string | Yes | Path to executable (relative to plugin root) |

### [[hooks]] Entries

Hooks register binaries that run in response to fledge events.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `event` | string | Yes | Event to hook into (e.g., `lane:post`) |
| `binary` | string | Yes | Path to executable (relative to plugin root) |

## File Locations

| Path | Purpose |
|------|---------|
| `~/.config/fledge/plugins/` | Installed plugin directories |
| `~/.config/fledge/plugins/bin/` | Symlinked command binaries |
| `~/.config/fledge/plugins.toml` | Plugin registry (tracks installs) |
