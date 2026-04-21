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

### [[hooks]]

Hooks fire in response to fledge events.

| Field | Type | Required | |
|-------|------|----------|-|
| `event` | string | Yes | Event to hook (e.g. `lane:post`) |
| `binary` | string | Yes | Path to executable (relative to plugin root) |

## File Locations

| Path | What's there |
|------|-------------|
| `~/.config/fledge/plugins/` | Installed plugin directories |
| `~/.config/fledge/plugins/bin/` | Symlinked binaries |
| `~/.config/fledge/plugins.toml` | Plugin registry |
